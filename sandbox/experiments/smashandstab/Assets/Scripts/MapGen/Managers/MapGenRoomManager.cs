using System;
using System.Linq;
using Unity.Netcode;
using UnityEngine;

public class MapGenRoomManager : MonoBehaviour {

    // A default sprite will be used for rooms which don't have one, if it exists.
    [SerializeField]
    private Sprite defaultModuleSprite;

    [NonSerialized]
    public MapGenerator mapGenerator;

    // If possible, adds a room at the position specified.
    public bool TryAddRoom(MapGenRoom roomToAdd, Vector2 roomModularPos)
    {
        if (CanAddRoom(roomToAdd, roomModularPos))
        {
            AddRoom(roomToAdd, roomModularPos);
            return true;
        }
        return false;
    }

    // Check if modules of the room at this position would overlap with used modules.
    private bool CanAddRoom(MapGenRoom roomToCheckAdding, Vector2 roomModularPos)
    {
        foreach (Vector2 modulePos in roomToCheckAdding.localModulePosList)
        {
            if (mapGenerator.usedModulePosSet.Contains(modulePos + roomModularPos))
                return false;
            else if (!ModulePosInBounds(modulePos + roomModularPos))
                return false;
        }

        return true;
    }

    // Intended to be used alongside CanAddRoom.
    private void AddRoom(MapGenRoom roomToAdd, Vector2 roomModularPos)
    {
        float distanceFactor;
        MapGenRoom.Connector globalConnector;

        roomToAdd.modularPos = roomModularPos;
        mapGenerator.usedRoomSet.Add(roomToAdd.Clone());
        
        foreach (Vector2 modulePosTmp in roomToAdd.localModulePosList)
            mapGenerator.usedModulePosSet.Add(modulePosTmp + roomModularPos);

        // Add all connectors.
        // Since the connector list is updated after, there's no need to validate this until then.
        foreach (MapGenRoom.Connector connectorTmp in roomToAdd.localConnectorList)
        {
            globalConnector = new MapGenRoom.Connector
            {
                modularPos = connectorTmp.modularPos + roomModularPos,
                direction = connectorTmp.direction
            };
            
            distanceFactor = mapGenerator.mapGenHyperParams.initRoom.DistanceToModule(globalConnector.targetModulePos);
            globalConnector.weight = (float)Math.Pow(distanceFactor, -mapGenerator.mapGenHyperParams.weightTowardInitRoomOrder);

            mapGenerator.usableConnectorSet.Add(globalConnector);
            mapGenerator.existingConnectorSet.Add(globalConnector);
        }

        UpdateConnectorList();


        if (roomToAdd.sprite != null)
        {
            roomToAdd.GetModularSizeAndLocalModularCenter(out Vector2 modularSize, out Vector2 modularLocalCenter);

            AddRoomSprite(roomToAdd,
                          modularSize * mapGenerator.moduleSizeUnits, 
                          (roomModularPos + modularLocalCenter + new Vector2(0.5f, 0.5f)) * mapGenerator.moduleSizeUnits);
        }
        else if (defaultModuleSprite != null)
        {
            // This will basically add the default room sprite for each module.
            foreach (Vector2 localModulePosTmp in roomToAdd.localModulePosList)
            {
                AddRoomSprite(roomToAdd,
                              mapGenerator.moduleSizeUnits, 
                              (roomModularPos + localModulePosTmp + new Vector2(0.5f, 0.5f)) * mapGenerator.moduleSizeUnits);
            }
        }
    }

    // Adds the sprite of a room.
    private void AddRoomSprite(MapGenRoom roomToAdd, Vector2 size, Vector2 center)
    {
        GameObject gameObjectWithSprite = new();

        SpriteRenderer spriteRenderer = gameObjectWithSprite.AddComponent<SpriteRenderer>();
        spriteRenderer.sprite = roomToAdd.sprite == null ? defaultModuleSprite : roomToAdd.sprite;
        spriteRenderer.drawMode = SpriteDrawMode.Sliced;
        spriteRenderer.size = size;

        gameObjectWithSprite.name = "Sprite GameObject (" + roomToAdd.name + ", " + center + ")";
        gameObjectWithSprite.transform.position = center;

        gameObjectWithSprite.transform.SetParent(this.transform);
        gameObjectWithSprite.AddComponent<MapGenSpawnedObject>();
    }

    // This will check all connectors in usableConnectorSet to see if they either lead to a used module,
    // or out of bounds. In both cases the connector is removed from the list. existingConnectorSet is intentionally unaffected.
    private void UpdateConnectorList()
    {
        Vector2 conTargetModulePos;
        foreach (MapGenRoom.Connector con in mapGenerator.usableConnectorSet.ToArray())
        {
            conTargetModulePos = con.targetModulePos;
            if (mapGenerator.usedModulePosSet.Contains(conTargetModulePos))
                mapGenerator.usableConnectorSet.Remove(con);
                
            if (!ModulePosInBounds(conTargetModulePos))
                mapGenerator.usableConnectorSet.Remove(con);
        }
    }

    // Self-explanatory.
    private bool ModulePosInBounds(Vector2 modulePos)
    {
        if ((modulePos.x > mapGenerator.mapGenHyperParams.mapModularMax.x) || (modulePos.x < mapGenerator.mapGenHyperParams.mapModularMin.x)
         || (modulePos.y > mapGenerator.mapGenHyperParams.mapModularMax.y) ||(modulePos.y < mapGenerator.mapGenHyperParams.mapModularMin.y))
            return false;
        return true;
    }
}
