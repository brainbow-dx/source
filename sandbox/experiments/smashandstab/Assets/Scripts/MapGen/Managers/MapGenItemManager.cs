using System;
using System.Collections.Generic;
using Unity.Netcode;
using UnityEngine;

public class MapGenItemManager : NetworkBehaviour
{
    // Crate. Used to cover items.
    [SerializeField]
    private GameObject crateObject;

    [NonSerialized]
    public MapGenerator mapGenerator;

    // Adds objects for each room. 
    public void ServerAddRoomObjects()
    {
        Vector3 unitPosTmp;
        GameObject gameObjectTmp;
        SpriteRenderer spriteRenderer;
        BoxCollider2D boxCollider2D;
        NetworkObject networkObjectTmp;
        List<MapGenRandomSpawnParams.WeightedObject> spawnableWeightedObjects;

        if (!NetworkManager.Singleton.IsServer)
            return;

        foreach (MapGenRoom roomToAdd in mapGenerator.usedRoomSet)
        {
            // Spawn hardcoded room objects.
            foreach (MapGenRoom.TilePositionedObject tilePositionedObjectTmp in roomToAdd.particularObjectSpawnPosList)
            {
                if (tilePositionedObjectTmp.theObject == null)
                {
                    continue;
                }
                gameObjectTmp = Instantiate(tilePositionedObjectTmp.theObject);
                unitPosTmp = (
                            (roomToAdd.modularPos + tilePositionedObjectTmp.modulePosition) * mapGenerator.moduleSizeTiles // Modules to tiles
                           + tilePositionedObjectTmp.tilePosition + new Vector2(0.5f, 0.5f)) * mapGenerator.tileSizeUnits; // Tiles to units
                                
                unitPosTmp += new Vector3(0, 0, -0.5f);
                gameObjectTmp.transform.position = unitPosTmp;

                if (gameObjectTmp.TryGetComponent<NetworkObject>(out networkObjectTmp))
                    networkObjectTmp.Spawn();
                else
                    Debug.LogError("One of the prefabs in " + nameof(roomToAdd.particularObjectSpawnPosList) + " doesn't have a " + nameof(NetworkObject) + " component, and can't spawn properly.");

                gameObjectTmp.transform.SetParent(this.transform);
                gameObjectTmp.AddComponent<MapGenSpawnedObject>();
            }

            // Return if there are none of these. Shouldn't be the case, though.
            // ...What a convoluted path.
            spawnableWeightedObjects = mapGenerator.mapGenHyperParams.mapGenRandomSpawnParams.spawnableWeightedObjects;
            if (spawnableWeightedObjects.Count <= 0)
                return;
        
            float totalWeight;
            float randThreshold;
            foreach (MapGenRoom.ModuleTilePosition moduleAndTileSpawnTmp in roomToAdd.randomObjectSpawnPosList)
            {
                gameObjectTmp = null;
                totalWeight = 0f;
                // Both these loops are used to get a random weighted object.
                foreach (MapGenRandomSpawnParams.WeightedObject weightedObjectTmp in spawnableWeightedObjects)
                {
                    totalWeight += weightedObjectTmp.weight;
                }
                randThreshold = UnityEngine.Random.Range(0f, totalWeight);
                foreach (MapGenRandomSpawnParams.WeightedObject weightedObjectTmp in spawnableWeightedObjects)
                {
                    randThreshold -= weightedObjectTmp.weight;
                    if (randThreshold < 0)
                    {
                        gameObjectTmp = weightedObjectTmp.theObject;
                        break;
                    }
                }

                // This allows you to "weight" nothing. A crate also won't spawn.
                if (gameObjectTmp == null)
                    continue;

                gameObjectTmp = Instantiate(gameObjectTmp);

                unitPosTmp = ((roomToAdd.modularPos + moduleAndTileSpawnTmp.modulePosition) * mapGenerator.moduleSizeTiles // Modules to tiles
                             + moduleAndTileSpawnTmp.tilePosition + new Vector2(0.5f, 0.5f)) * mapGenerator.tileSizeUnits; // Tiles to units

                unitPosTmp += new Vector3(0, 0, -0.5f);
                gameObjectTmp.transform.position = unitPosTmp;

                if (gameObjectTmp.TryGetComponent<NetworkObject>(out networkObjectTmp))
                    networkObjectTmp.Spawn();
                else
                    Debug.LogError("One of the prefabs in " + nameof(mapGenerator.mapGenHyperParams.mapGenRandomSpawnParams.spawnableWeightedObjects) + " doesn't have a " + nameof(NetworkObject) + " component, and can't spawn properly.");

                gameObjectTmp.transform.SetParent(this.transform);
                gameObjectTmp.AddComponent<MapGenSpawnedObject>();
                gameObjectTmp.name += " (" + roomToAdd.name + ", " + moduleAndTileSpawnTmp + ")";

                //This will also add a crate object, if one exists.
                if (crateObject != null)
                {
                    gameObjectTmp = Instantiate(crateObject);

                    unitPosTmp.z = -0.8f;
                    gameObjectTmp.transform.position = unitPosTmp;

                    if (gameObjectTmp.TryGetComponent<NetworkObject>(out networkObjectTmp))
                        networkObjectTmp.Spawn();
                    else
                        Debug.LogError(nameof(crateObject) + " prefab doesn't have a " + nameof(NetworkObject) + " component, and can't spawn properly.");

                    gameObjectTmp.transform.SetParent(this.transform);
                    gameObjectTmp.AddComponent<MapGenSpawnedObject>();

                    boxCollider2D = gameObjectTmp.GetComponent<BoxCollider2D>();
                    spriteRenderer = gameObjectTmp.GetComponent<SpriteRenderer>();

                    gameObjectTmp.name += " (" + roomToAdd.name + ", " + moduleAndTileSpawnTmp + ")";
                }
            }
        }
    }
}
