using UnityEngine;
using System.Collections.Generic;
using System.Linq;
using System;
using Unity.Netcode;

[RequireComponent(typeof(NetworkObject))]
public class MapGenerator : NetworkBehaviour
{
    [SerializeField]
    public float tileSizeUnits;
    
    [SerializeField]
    public Vector2 moduleSizeTiles;
    
    [SerializeField]
    public Vector2 connectorSizeTiles; // X is for connectors pointing up/down (horizontal), Y is for the left/right (vertical) ones.

    [NonSerialized]
    public Vector2 moduleSizeUnits; // = moduleSizeTiles * tileSizeUnits, set when beginning generation.
    [NonSerialized]
    public Vector2 connectorSizeUnits; // = connectorSizeTiles * tileSizeUnits, also set when beginning generation.

    [SerializeField]
    public float wallSizeTiles;

    // Max number of times this should attempt to generate a map.
    [SerializeField]
    private int maxGenAttempts;

    // MapGenParams contains multiple variables for map generation, so go check that!
    [SerializeField]
    public MapGenHyperParams mapGenHyperParams;


    [SerializeField]
    private MapGenItemManager mapGenItemManagerPrefab;

    

    // Handles rooms, parents their sprites.
    private MapGenRoomManager mapGenRoomManager;

    // Handles and parents items.
    private MapGenItemManager mapGenItemManager;

    // Handles and parents wall collisions.
    private MapGenCollisionManager mapGenCollisionManager;



    // An array of all rooms that can be used for map generation. Is filled at the start of TryGenerateMap.
    private MapGenRoom[] usableRoomArray;

    // A set of global connectors that can be used to append rooms. Emptied at the start of TryGenerateMap; connectors are added and removed every time a room gets added.
    public HashSet<MapGenRoom.Connector> usableConnectorSet;

    // A set of every connector that's been added to the map. Like usableConnectorSet, but this includes connectors that have already been used.
    public HashSet<MapGenRoom.Connector> existingConnectorSet;

    // A list of every room that's been added. Used to add collision at the end of TryGenerateMap.
    public HashSet<MapGenRoom> usedRoomSet;

    // A list of every module position that's in use. Used when checking to see if a room can be added. Is also used when generating collision.
    public HashSet<Vector2> usedModulePosSet;

    // Will run if an object with this script is present when Play is pressed.
    public void Start()
    {
        // There's a canvas with a button for map generation attached. If this isn't the server, destroy it.
        if (!IsServer)
        {
            Destroy(GetComponentInChildren<Canvas>().gameObject);
            return;
        }
        
        mapGenItemManager = Instantiate(mapGenItemManagerPrefab);
        mapGenItemManager.GetComponent<NetworkObject>().Spawn();
        mapGenItemManager.transform.SetParent(this.transform);

        BeginGeneration();
    }

    public void BeginGeneration()
    {
        if (!IsServer)
            return;

        // The server picks a random seed and sends it to all clients for map generation.
        int seed = UnityEngine.Random.Range(int.MinValue, int.MaxValue);
        BeginGenerationClientRpc(seed);
    }
    
    // Will repeatedly run the same function until it either succeeds, reaches the limit, or runs into an error.
    [ClientRpc]
    public void BeginGenerationClientRpc(int seed)
    {
        int iteration = 1;
        bool tryAgain;

        UnityEngine.Random.InitState(seed);

        TrySetManagerFields();

        // This is absolutely needed for generation to function, else it doesn't work at all.
        if (mapGenHyperParams == null)
        {
            throw new Exception(nameof(mapGenHyperParams) + "of this " + nameof(MapGenerator) + " component is null!");
        }

        moduleSizeUnits = moduleSizeTiles * tileSizeUnits;
        connectorSizeUnits = connectorSizeTiles * tileSizeUnits;

        // You can ignore this. It's a completely unnecessary function depending on whether the hyper params use RNG.
        mapGenHyperParams.RunRandomization();

        // Continuously attempt to generate a map until it succeeds or gives up.
        while (true)
        {
            TryGenerateMap(out tryAgain);
            if (!tryAgain)
            {
                Debug.Log("Successfully generated map. Attempts: " + iteration + ".");
                break;
            }
            else
            {
                if (iteration >= maxGenAttempts)
                {
                    Debug.LogError("Failed to generate map. Attempts: " + iteration + ".");
                    break;
                }
                iteration++;
            }
        }
    }

    private void TrySetManagerFields()
    {
        mapGenRoomManager = GetComponentInChildren<MapGenRoomManager>();
        mapGenItemManager = GetComponentInChildren<MapGenItemManager>();
        mapGenCollisionManager = GetComponentInChildren<MapGenCollisionManager>();
        if (mapGenRoomManager == null || mapGenItemManager == null || mapGenCollisionManager == null)
        {
            throw new Exception("A " + nameof(MapGenRoomManager) + ", " + nameof(MapGenItemManager) + ", and " + nameof(MapGenCollisionManager)
                                 + " are all required as components in children of this " + nameof(MapGenerator) + " component");
        }

        mapGenRoomManager.mapGenerator = this;
        mapGenItemManager.mapGenerator = this;
        mapGenCollisionManager.mapGenerator = this;
    }

    // A complicated function. Is basically the heart of map generation.
    private void TryGenerateMap(out bool tryAgain)
    {
        MapGenRoom roomToAppend;
        MapGenRoom.Connector connectorToUse;
        MapGenRoom.Connector connectorToAppend;
        Vector2 roomModularPosToTry;
        float weightMultFactor;
        
        // Reset some sets.
        usedModulePosSet = new();
        usedRoomSet = new();
        usableConnectorSet = new();
        existingConnectorSet = new();

        // Removes spawned objects between iterations.
        DespawnMapGeneratorSpawnedObjects();

        // Will begin by adding the initial room. This shouldn't fail, but it would be critical if it did.
        if (!mapGenRoomManager.TryAddRoom(mapGenHyperParams.initRoom, mapGenHyperParams.mapModularStart))
        {
            throw new Exception("Failed to add initial room.");
        }

        // Initialize the array of usable rooms by filling it with normal and special rooms.
        List<MapGenRoom> roomListTmp = GetRooms("Rooms").ToList();
        roomListTmp.AddRange(GetRooms("SpecialRooms"));
        usableRoomArray = roomListTmp.ToArray();

        if (usableRoomArray.Length <= 0)
        {
            throw new Exception("No rooms found in Resources/Rooms.");
        }

        // This loop is for adding rooms directly next to other rooms, starting from the initial room.
        // Vaguely similar to random walk, but it's a bit more organized.
        for (int i = 1; i <= mapGenHyperParams.addRoomLoopMax; i++)
        {
            // Get a random connector using weights. If this returns false, there are none, so the loop should end.
            if (!MapGenRoom.TryGetRandomConnectorArrayElement(usableConnectorSet.ToArray(), out connectorToUse, true))
                break;

            // Adjust the weights of rooms with the type GENERATE_NORTH based on the connector's y-position.
            // This basically means GENERATE_NORTH rooms are more likely to be selected the higher the y-coordinate is.
            foreach (MapGenRoom roomTmp in usableRoomArray)
            {
                if (roomTmp.roomFlags.HasFlag(MapGenRoom.RoomFlags.GENERATE_NORTH))
                {
                    roomTmp.GetLocalModularMaxAndMin(out _, out Vector2 localMax);
                    // I know this code looks sketchy, but the farther north this room would go, the higher its chances of being picked are.
                    weightMultFactor = 3 * (connectorToUse.targetModulePos.y + localMax.y + 1 - mapGenHyperParams.mapModularMax.y);
                    if (weightMultFactor < 0)
                        weightMultFactor = 0;

                    roomTmp.curWeight = roomTmp.baseWeight * weightMultFactor;
                }
            }
            // Get a random room. If this returns false, there are none, so the loop should end.
            if (!MapGenRoom.TryGetRandomRoomArrayElement(usableRoomArray, out roomToAppend, true))
                break;

            // Get a connector belonging to the prior room going in the opposite direction of the prior connector.
            // If this returns false, there are none- but the looping can continue in this case.
            if (!roomToAppend.TryGetRandomConnectorOfDirection(-connectorToUse.directionVector, out connectorToAppend, false))
                continue;

            roomModularPosToTry = connectorToUse.modularPos + connectorToUse.directionVector - connectorToAppend.modularPos;

            // Will attempt to add a room here.
            if (mapGenRoomManager.TryAddRoom(roomToAppend, roomModularPosToTry))
            {
                // A room that only generates once will be removed.
                if (roomToAppend.roomFlags.HasFlag(MapGenRoom.RoomFlags.GENERATE_ONCE))
                {
                    roomListTmp = usableRoomArray.ToList();
                    roomListTmp.Remove(roomToAppend);
                    usableRoomArray = roomListTmp.ToArray();
                }
            }

        }



        // Client RNG is done, so server RNG can run safely. Particularly important for spawning objects.

        // Add room objects. This is a server-only job.
        mapGenItemManager.ServerAddRoomObjects();

        // Add collision.

        if (mapGenCollisionManager != null)
            mapGenCollisionManager.AddCollision();
        else
            Debug.LogError(nameof(mapGenCollisionManager) + "of this " + nameof(MapGenerator) + " component is null!");

        // This checks to see if any rooms in the room array have the GENERATE_ONCE flag. If so, that means they weren't actually generated,
        // else they'd have been removed. Try again in this case.
        tryAgain = false;
        foreach (MapGenRoom roomTmp in usableRoomArray)
        {
            if (roomTmp.roomFlags.HasFlag(MapGenRoom.RoomFlags.GENERATE_ONCE))
            {
                tryAgain = true;
                return;
            }
        }
    }

    // Gets all rooms at a certain path from Resources, and places them in an array.
    public static MapGenRoom[] GetRooms(string path){
        List<MapGenRoom> roomList = new();

        // By adding cloned rooms, making modifications to said rooms won't affect the original scriptable object.
        foreach (MapGenRoom room in Resources.LoadAll<MapGenRoom>(path).ToList())
            roomList.Add(room.Clone());

        return roomList.ToArray<MapGenRoom>();
    }

    // Finds objects with the component for objects spawned by the map generator, and basically despawns/destroys every one.
    public void DespawnMapGeneratorSpawnedObjects()
    {
        NetworkObject networkObjectTmp;
        GameObject gameObjectTmp;
        foreach (MapGenSpawnedObject mapGenSpawnedObject in FindObjectsOfType<MapGenSpawnedObject>())
            if ((gameObjectTmp = mapGenSpawnedObject.gameObject) != null)
            {
                if (gameObjectTmp.TryGetComponent<NetworkObject>(out networkObjectTmp))
                {
                    if (networkObjectTmp.IsSpawned)
                    {
                        networkObjectTmp.Despawn();
                    }
                }
                else
                    Destroy(gameObjectTmp);
            }
    }
}