using System;
using UnityEngine;

[CreateAssetMenu(fileName = nameof(MapGenHyperParams), menuName = "MapGen/" + nameof(MapGenHyperParams))]
public class MapGenHyperParams : ScriptableObject
{
    public bool randomizesInitRoom;
    // Randomization variables for the lunatics out there :)
    public bool randomizesModuleBoundsAndStart;
    public bool randomizesSize;
    public bool randomizesCenterWeight;

    // The initial room to be added for map generation.
    public MapGenRoom initRoom;

    // The modular start position and bounds of map generation.
    public Vector2 mapModularMin;
    public Vector2 mapModularMax;
    public Vector2 mapModularStart;

    // Number of times for the loop in which rooms are added to occur.
    public int addRoomLoopMax;

    // The higher this is, the more often rooms will be added closer to the center.
    public double weightTowardInitRoomOrder;   

    public MapGenRandomSpawnParams mapGenRandomSpawnParams;

    public void RunRandomization()
    {
        if (randomizesInitRoom)
        {
            MapGenRoom[] usableRoomArray = MapGenerator.GetRooms("InitialRooms");

            if (!MapGenRoom.TryGetRandomRoomArrayElement(usableRoomArray, out MapGenRoom roomToAppend, true))
            {
                throw new Exception("No rooms found in Resources/InitialRooms.");
            }

            Debug.Log(roomToAppend.baseWeight);
        
            initRoom = roomToAppend;

            Debug.Log(initRoom.baseWeight);
        }

        if (randomizesModuleBoundsAndStart)
        {
            mapModularMin.x = UnityEngine.Random.Range(-20, 10);
            mapModularMin.y = UnityEngine.Random.Range(-20, 10);
            mapModularMax.x = mapModularMin.x + UnityEngine.Random.Range(8, 20);
            mapModularMax.y = mapModularMin.y + UnityEngine.Random.Range(8, 20);
            mapModularStart.x = UnityEngine.Random.Range((int)mapModularMin.x + 1, (int)mapModularMax.x - 5);
            mapModularStart.y = UnityEngine.Random.Range((int)mapModularMin.y + 1, (int)mapModularMax.y - 5);
        }

        if (randomizesSize)
        {
            addRoomLoopMax = (int)Math.Pow(2, UnityEngine.Random.Range(5f, 10f));
        }

        if (randomizesCenterWeight)
        {
            weightTowardInitRoomOrder = UnityEngine.Random.Range(-3f, 3f);
        }
    }
}