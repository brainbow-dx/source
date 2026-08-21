using UnityEngine;
using System.Collections.Generic;
using System;

// Has a sprite with specified module positions and connectors.
[CreateAssetMenu(fileName = nameof(MapGenRoom), menuName = "MapGen/" + nameof(MapGenRoom))]
public class MapGenRoom : ScriptableObject
{
    // When modifying any of these variables, you also need to update Clone(). Is there a better way to do this?

    public RoomFlags roomFlags;
    public Sprite sprite;
    public float baseWeight;
    public List<Vector2> localModulePosList;
    public List<Connector> localConnectorList;
    public List<ModuleTilePosition> randomObjectSpawnPosList;
    public List<TilePositionedObject> particularObjectSpawnPosList;

    [Serializable]
    public class TilePositionedObject
    {
        public GameObject theObject;
        public Vector2 modulePosition;
        public Vector2 tilePosition;
    }

    [Serializable]
    public struct ModuleTilePosition
    {
        public Vector2 modulePosition;
        public Vector2 tilePosition;
    }

    // Note that curWeight needs to be set through code. This only isn't a problem
    // because the Clone function sets  curWeight of the newly created room, which is what MapGenerator uses.
    [NonSerialized] public float curWeight;
    [NonSerialized] public Vector2 modularPos;

    public MapGenRoom Clone()
    {
        MapGenRoom clonedRoom = CreateInstance<MapGenRoom>();
        clonedRoom.name = name;
        clonedRoom.roomFlags = roomFlags;
        clonedRoom.sprite = sprite;
        clonedRoom.curWeight = clonedRoom.baseWeight = baseWeight;
        clonedRoom.localModulePosList = localModulePosList;
        clonedRoom.localConnectorList = localConnectorList;
        clonedRoom.modularPos = modularPos;
        clonedRoom.randomObjectSpawnPosList = randomObjectSpawnPosList;
        clonedRoom.particularObjectSpawnPosList = particularObjectSpawnPosList;
        return clonedRoom;
    }

    [Flags]
    public enum RoomFlags : uint
    {
        GENERATE_ONCE = 0x0001,
        GENERATE_NORTH = 0x0002,
    }
    
    public static readonly HashSet<Vector2> connectorDirections = new()
    {
        Vector2.up,
        Vector2.right,
        Vector2.down,
        Vector2.left,
    };

    // Connectors are used to connect rooms together (and probably other stuff later).
    [Serializable]
    public struct Connector
    {
        public Vector2 modularPos;

        public Direction direction;

        // Controls the chances of this connector being picked out at random.
        [NonSerialized]
        public float weight;

        public readonly Vector2 directionVector => direction switch
        {
            Direction.Up => Vector2.up,
            Direction.Down => Vector2.down,
            Direction.Left => Vector2.left,
            Direction.Right => Vector2.right,
            _ => Vector2.zero,
        };

        public readonly Vector2 targetModulePos => modularPos + directionVector;
    }

    [Serializable]
    public enum Direction
    {
        Up, Down, Left, Right
    }

    [Serializable]
    public struct TilePositionedGameObject
    {
        public ItemDefinition collectible;
        public Vector2 tilePos;
    }

    public void GetModularSizeAndLocalModularCenter(out Vector2 size, out Vector2 center)
    {
        // Get the min/max modular positions of this room.
        GetLocalModularMaxAndMin(out Vector2 min, out Vector2 max);


        // Size X and Y are one more than the difference between min and max X and Y respectively+, while center is the average of min and max.
        size = new Vector2(max.x - min.x + 1f, max.y - min.y + 1f);
        center = new Vector2((max.x + min.x)/2f, (max.y + min.y)/2f);
    }

    public void GetLocalModularMaxAndMin(out Vector2 min, out Vector2 max)
    {
        min = new Vector2(float.PositiveInfinity, float.PositiveInfinity);
        max = new Vector2(float.NegativeInfinity, float.NegativeInfinity);

        foreach (Vector2 modulePos in localModulePosList)
        {

            if (modulePos.x < min.x)
                min.x = modulePos.x;
            if (modulePos.x > max.x)
                max.x = modulePos.x;
            if (modulePos.y < min.y)
                min.y = modulePos.y;
            if (modulePos.y > max.y)
                max.y = modulePos.y;
        }
    }

    public float DistanceToModule(Vector2 posInput)
    {
        float distMin = float.PositiveInfinity;
        float distTmp;
        Vector2 dispToInputPos = posInput - this.modularPos;

        foreach (Vector2 modulePosLocal in this.localModulePosList)
        {
            distTmp = (dispToInputPos - modulePosLocal).magnitude;
            if (distTmp < distMin)
                distMin = distTmp;
        }

        return distMin;
    }

    /*
    public float DistanceToRoom(Room roomOther, Vector2 posInput)
    {
        float distMin = float.PositiveInfinity;
        float distTmp;
        Vector2 dispToInputPos = posInput - this.modularPos;

        foreach (Vector2 modulePosLocalToThis in this.localModulePosList)
        {
            foreach (Vector2 modulePosLocalToOther in roomOther.localModulePosList)
            {
                distTmp = (dispToInputPos - modulePosLocalToThis + modulePosLocalToOther).magnitude;
                if (distTmp < distMin)
                    distMin = distTmp;
            }
        }

        return distMin;
    }
    */

    //Pulls a room at random.
    public static bool TryGetRandomRoomArrayElement(MapGenRoom[] roomArray, out MapGenRoom roomOut, bool weighted)
    {
        if (roomArray.Length <= 0)
        {
            roomOut = default;
            return false;
        }
        
        if (!weighted)
        {
            // Very simple. Get a random value whose range spans all array index values,
            // then get the room at that index.
            int randIndex = UnityEngine.Random.Range(0, roomArray.Length);
            roomOut = roomArray[randIndex];
            return true;
        }
        else
        {
            // This will pick a random float between 0 and the total weight.
            // It will decrease that float by weights one after another until it's less/equal to zero.
            // Thus, the more weight a room has, the higher the chances are it'll be picked out.
            // Additionally, a room will always be picked out this way.
            float totalWeight = 0f;
            float randThreshold;
            foreach (MapGenRoom roomTmp in roomArray)
            {
                totalWeight += roomTmp.curWeight;
            }
            randThreshold = UnityEngine.Random.Range(0f, totalWeight);
            foreach (MapGenRoom roomTmp in roomArray)
            {
                randThreshold -= roomTmp.curWeight;
                if (randThreshold > 0)
                    continue;
                roomOut = roomTmp;
                return true;
            }
        }
        roomOut = new();
        return false;
    }

    //Pulls a connector at random; is virtually the same as TryGetRandomRoomArrayElement but with connectors instead.
    public static bool TryGetRandomConnectorArrayElement(Connector[] connectorArray, out Connector connectorOut, bool weighted)
    {
        if (connectorArray.Length <= 0)
        {
            connectorOut = default;
            return false;
        }
        
        if (!weighted)
        {
            int randIndex = UnityEngine.Random.Range(0, connectorArray.Length);
            connectorOut = connectorArray[randIndex];
            return true;
        }
        else
        {
            float totalWeight = 0f;
            float randThreshold;
            foreach (Connector connectorTmp in connectorArray)
            {
                totalWeight += connectorTmp.weight;
            }
            randThreshold = UnityEngine.Random.Range(0f, totalWeight);
            foreach (Connector connectorTmp in connectorArray)
            {
                randThreshold -= connectorTmp.weight;
                if (randThreshold > 0)
                    continue;
                connectorOut = connectorTmp;
                return true;
            }
        }
        connectorOut = new();
        return false;
    }

    // Gets a random connector of the specified direction from the specified room.
    public bool TryGetRandomConnectorOfDirection(Vector2 direction, out Connector connectorOut, bool weighted) {
        List<Connector> connectorListTmp = new();
        foreach (Connector connectorTmp in localConnectorList)
        {
            if (connectorTmp.directionVector == direction)
                connectorListTmp.Add(connectorTmp);
        }

        if (!TryGetRandomConnectorArrayElement(connectorListTmp.ToArray(), out connectorOut, weighted))
            return false;

        return true;
    }
}