using System;
using System.Collections.Generic;
using UnityEngine;
using Unity.VisualScripting;

// This is its own file only because it's 150 lines of clutter if it was in MapGenerator instead.

public class MapGenCollisionManager : MonoBehaviour
{
    [SerializeField]
    private Material collisionMaterial;

    [SerializeField]
    private Sprite collisionSprite;

    [SerializeField]
    private bool drawNorthWalls;

    [SerializeField]
    private bool drawThinWalls;

    [NonSerialized]
    public MapGenerator mapGenerator;

    private Vector2 offsetModBottomToVertConnBottom,
            offsetModBottomToVertConnTop,
            offsetModLeftToHorConnLeft,
            offsetModLeftToHorConnRight,

            offsetModBottomToModTop,
            offsetModLeftToModRight,

            offsetModBottomToWallBottom;

    private List<CollisionBox> collisionBoxList;
    private List<CollisionEdge> collisionEdgeList;

    private class CollisionBox
    {
        public float left, right, top, bottom;
    }
    private class CollisionEdge
    {
        public Vector2 visualCorner0, visualCorner1;
        public Vector2 collisionCorner0, collisionCorner1;
        public bool ignoreCollision; // false by default
    }
    
    public void AddCollision()
    {
        bool connector1Exists, connector2Exists;
        Vector2 globalModulePosTmp;

        UpdateFields();

        // For each room, for each module, collision will be added to its sides under certain conditions.
        foreach (MapGenRoom roomTmp in mapGenerator.usedRoomSet)
        {
            foreach (Vector2 localModulePosTmp in roomTmp.localModulePosList)
            {
                globalModulePosTmp = localModulePosTmp + roomTmp.modularPos;
                foreach (Vector2 directionTmp in MapGenRoom.connectorDirections)
                {
                    // If the adjacent module (global) is not used, draw collision along the edge between modules.
                    if (!mapGenerator.usedModulePosSet.Contains(globalModulePosTmp + directionTmp))
                        AppendModularCollisionToGameObject(globalModulePosTmp, directionTmp, true);
                    else
                    {
                        // Adjacent module would be in use.

                        // Skip if the current module and adjacent module (local) are part of the same room.
                        if (roomTmp.localModulePosList.Contains(localModulePosTmp + directionTmp))
                            continue;

                        // The left/right edge between used modules is shared. It will only be drawn from the left, and not the right. This prevents double-stacking collision.
                        if (directionTmp == Vector2.right)
                            continue;

                        // Iterate through existing connectors. Check that one exists at the module position going in the given direction
                        // and another at the first connector's target pos going in the opposite direction.
                        connector1Exists = connector2Exists = false;
                        foreach (MapGenRoom.Connector connectorTmp in mapGenerator.existingConnectorSet)
                        {
                            if (connectorTmp.modularPos == globalModulePosTmp && connectorTmp.directionVector == directionTmp)
                                connector1Exists = true;

                            if (connectorTmp.modularPos == globalModulePosTmp + directionTmp && connectorTmp.directionVector == -directionTmp)
                                connector2Exists = true;
                        }

                        // If so, draw connector collision between these modules.
                        if (connector1Exists && connector2Exists)
                        {
                            AppendModularCollisionToGameObject(globalModulePosTmp, directionTmp, false);
                        }
                        // If not, draw collision along the edge between modules. Same as before.
                        else
                            AppendModularCollisionToGameObject(globalModulePosTmp, directionTmp, true);
                    }
                }
            }
        }

        MinimizeCollision();

        foreach (CollisionBox collisionBoxTmp in collisionBoxList)
        {
            CreateBox(collisionBoxTmp);
        }

        foreach (CollisionEdge collisionEdgeTmp in collisionEdgeList)
        {
            CreateEdge(collisionEdgeTmp);
        }
    }

    public void UpdateFields()
    {
        collisionBoxList = new();
        collisionEdgeList = new();

        offsetModBottomToVertConnBottom = Vector2.up * (mapGenerator.moduleSizeUnits.y - mapGenerator.connectorSizeUnits.y) / 2;
        offsetModBottomToVertConnTop = Vector2.up * (mapGenerator.moduleSizeUnits.y + mapGenerator.connectorSizeUnits.y) / 2;
        offsetModLeftToHorConnLeft = Vector2.right * (mapGenerator.moduleSizeUnits.x - mapGenerator.connectorSizeUnits.x) / 2;
        offsetModLeftToHorConnRight = Vector2.right * (mapGenerator.moduleSizeUnits.x + mapGenerator.connectorSizeUnits.x) / 2;

        offsetModBottomToModTop = Vector2.up * mapGenerator.moduleSizeUnits.y;
        offsetModLeftToModRight = Vector2.right * mapGenerator.moduleSizeUnits.y;

        offsetModBottomToWallBottom = Vector2.up * (mapGenerator.moduleSizeTiles.y - mapGenerator.wallSizeTiles) * mapGenerator.tileSizeUnits;
    }

    private void AppendModularCollisionToGameObject(Vector2 modularPos, Vector2 direction, bool fullEdge)
    {
        // Makes the necessary size of this function smaller.
        if (direction == Vector2.right)
        {
            direction = Vector2.left;
            modularPos.x += 1;
        }

        Vector2 moduleUnitPosLL = modularPos * mapGenerator.moduleSizeUnits;

        // This is difficult and tedious to explain. You'd be better off interpreting it for yourself.
        if (fullEdge)
        {
            if (direction == Vector2.left)
            {
                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL,
                                                        moduleUnitPosLL + offsetModBottomToModTop));
            }
            if (direction == Vector2.down)
            {
                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL,
                                                        moduleUnitPosLL + offsetModLeftToModRight));
            }
            if (direction == Vector2.up)
            {
                collisionBoxList.Add(newBoxFromPoints(moduleUnitPosLL + offsetModBottomToWallBottom,
                                                      moduleUnitPosLL + offsetModLeftToModRight + offsetModBottomToModTop));
            }

        }
        else
        {
            if (direction == Vector2.left)
            {
                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL,
                                                        moduleUnitPosLL + offsetModBottomToVertConnBottom));

                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL + offsetModBottomToVertConnTop,
                                                        moduleUnitPosLL + offsetModBottomToModTop));
            }
            if (direction == Vector2.down)
            {
                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL,
                                                        moduleUnitPosLL + offsetModLeftToHorConnLeft));

                collisionEdgeList.Add(newEdgeFromPoints(moduleUnitPosLL + offsetModLeftToHorConnRight,
                                                        moduleUnitPosLL + offsetModLeftToModRight));
            }
            if (direction == Vector2.up)
            {
                collisionBoxList.Add(newBoxFromPoints(moduleUnitPosLL + offsetModBottomToWallBottom,
                                                      moduleUnitPosLL + offsetModBottomToModTop + offsetModLeftToHorConnLeft));

                collisionBoxList.Add(newBoxFromPoints(moduleUnitPosLL + offsetModBottomToWallBottom + offsetModLeftToHorConnRight,
                                                      moduleUnitPosLL + offsetModBottomToModTop + offsetModLeftToModRight));
            }
        }
    }

    private CollisionEdge newEdgeFromPoints(Vector2 point0, Vector2 point1)
    {
        Vector2 vectorTmp;

        // This basically swaps edge-points so 0 is left and 1 is right, or 0 is bottom and 1 is top.
        // Useful for some assumptions.
        if (point0.x > point1.x || point0.y > point1.y)
        {
            vectorTmp = point0;
            point0 = point1;
            point1 = vectorTmp;
        }

        return new CollisionEdge()
        {
            visualCorner0 = point0,
            collisionCorner0 = point0,
            visualCorner1 = point1,
            collisionCorner1 = point1,
        };
    }

    private CollisionBox newBoxFromPoints(Vector2 point0, Vector2 point1)
    {
        return new CollisionBox()
        {
            left = point0.x,
            bottom = point0.y,
            right = point1.x,
            top = point1.y,
        };
    }





    private void MinimizeCollision()
    {
        MinimizeBoxes();
        MinimizeEdges();
    }

    // This assumes no boxes are contained entirely inside other boxes,
    // and instead simply merges adjacent boxes with similar dimensions.
    private void MinimizeBoxes()
    {
        CollisionBox boxI, boxJ, newBox = default;
        bool combineBoxesThisLoop;

        for (int i = 1; i < collisionBoxList.Count; i++)
        {
            for (int j = 0; j < i; j++)
            {

                combineBoxesThisLoop = false;

                boxI = collisionBoxList[i];
                boxJ = collisionBoxList[j];

                if (boxI.top == boxJ.top && boxI.bottom == boxJ.bottom)
                {
                    if (boxI.left <= boxJ.left && boxI.right >= boxJ.left)
                    {
                        // New box from I left to J right
                        newBox = new CollisionBox
                        {
                            left = boxI.left,
                            right = boxJ.right,
                            top = boxI.top,
                            bottom = boxI.bottom
                        };
                        combineBoxesThisLoop = true;   
                    }
                    // Symmetrical to the previous branch, with boxI and boxJ swapped.
                    else if (boxJ.left <= boxI.left && boxJ.right >= boxI.left)
                    {
                        // New box from J left to I right
                        newBox = new CollisionBox
                        {
                            left = boxJ.left,
                            right = boxI.right,
                            top = boxJ.top,
                            bottom = boxJ.bottom
                        };
                        combineBoxesThisLoop = true;
                    }
                }
                // Symmetrical to the previous branch, with top and right swapped and bottom and left swapped.
                else if (boxI.left == boxJ.right && boxI.left == boxJ.right)
                {
                    if (boxI.bottom <= boxJ.bottom && boxI.top >= boxJ.bottom)
                    {
                        // New box from I bottom to J top
                        newBox = new CollisionBox
                        {
                            bottom = boxI.bottom,
                            top = boxJ.top,
                            right = boxI.right,
                            left = boxI.left
                        };
                        combineBoxesThisLoop = true;

                    }
                    // Symmetrical to the previous branch, with boxI and boxJ swapped.
                    else if (boxJ.bottom <= boxI.bottom && boxJ.top >= boxI.bottom)
                    {
                        // New box from J bottom to I top
                        newBox = new CollisionBox
                        {
                            bottom = boxJ.bottom,
                            top = boxI.top,
                            right = boxJ.right,
                            left = boxJ.left
                        };
                        combineBoxesThisLoop = true;
                    }
                }

                if (combineBoxesThisLoop)
                {
                    // New items are always added at the end of the list.
                    // Therefore, by comparing every item in the list with every item that came before,
                    // comparisons between new items and old ones shouldn't be missed.
                    // We also decrease j by 1 because the item at j was removed,
                    // and i by 2 because the item at i and another item before were removed.
                    collisionBoxList.Remove(boxI);
                    collisionBoxList.Remove(boxJ);
                    collisionBoxList.Add(newBox);
                    i -= 2;
                    j -= 1;
                }
            }
        }
    }

    // This assumes all edges are either horizontal or vertical, or that at least the ones to be combined are.
    // I know how to do diagonal ones as well, but I opted not to because it'll needlessly complicate things. Mainly due to float imprecision.
    private void MinimizeEdges()
    {
        CollisionEdge edgeI, edgeJ, newEdge = default;
        CollisionBox boxK;
        Vector2 intersection;

        bool combineEdgesThisLoop;
        for (int i = 1; i < collisionEdgeList.Count; i++)
        {
            for (int j = 0; j < i; j++)
            {

                combineEdgesThisLoop = false;

                edgeI = collisionEdgeList[i];
                edgeJ = collisionEdgeList[j];

                if (edgeI.collisionCorner0.y == edgeI.collisionCorner1.y
                 && edgeI.collisionCorner0.y == edgeJ.collisionCorner0.y
                 && edgeI.collisionCorner0.y == edgeJ.collisionCorner1.y)
                {
                    // STRONGLY assumes edges are sorted from left to right, bottom to top, and also that no edge completely overlaps another edge.
                    if (edgeI.collisionCorner0.x <= edgeJ.collisionCorner0.x)
                    {
                        if (edgeI.collisionCorner1.x >= edgeJ.collisionCorner0.x)
                        {
                            newEdge = new CollisionEdge()
                            {
                                visualCorner0 = edgeI.visualCorner0,
                                visualCorner1 = edgeJ.visualCorner1,
                                collisionCorner0 = edgeI.collisionCorner0,
                                collisionCorner1 = edgeJ.collisionCorner1,
                            };
                            combineEdgesThisLoop = true;
                        }
                    }
                    else
                    {
                        if (edgeJ.collisionCorner1.x >= edgeI.collisionCorner0.x)
                        {
                            newEdge = new CollisionEdge()
                            {
                                visualCorner0 = edgeJ.visualCorner0,
                                visualCorner1 = edgeI.visualCorner1,
                                collisionCorner0 = edgeJ.collisionCorner0,
                                collisionCorner1 = edgeI.collisionCorner1,
                            };
                            combineEdgesThisLoop = true;
                        }
                    }
                }
                else if (edgeI.collisionCorner0.x == edgeI.collisionCorner1.x
                 && edgeI.collisionCorner0.x == edgeJ.collisionCorner0.x
                 && edgeI.collisionCorner0.x == edgeJ.collisionCorner1.x)
                {
                    if (edgeI.collisionCorner0.y <= edgeJ.collisionCorner0.y)
                    {
                        if (edgeI.collisionCorner1.y >= edgeJ.collisionCorner0.y)
                        {
                            newEdge = new CollisionEdge()
                            {
                                visualCorner0 = edgeI.visualCorner0,
                                visualCorner1 = edgeJ.visualCorner1,
                                collisionCorner0 = edgeI.collisionCorner0,
                                collisionCorner1 = edgeJ.collisionCorner1,
                            };
                            combineEdgesThisLoop = true;
                        }
                    }
                    else
                    {
                        if (edgeJ.collisionCorner1.y >= edgeI.collisionCorner0.y)
                        {
                            newEdge = new CollisionEdge()
                            {
                                visualCorner0 = edgeJ.visualCorner0,
                                visualCorner1 = edgeI.visualCorner1,
                                collisionCorner0 = edgeJ.collisionCorner0,
                                collisionCorner1 = edgeI.collisionCorner1,
                            };
                            combineEdgesThisLoop = true;
                        }
                    }
                }



                if (combineEdgesThisLoop)
                {

                    collisionEdgeList.Remove(edgeI);
                    collisionEdgeList.Remove(edgeJ);
                    collisionEdgeList.Add(newEdge);
                    i -= 2;
                    j -= 1;
                }
            }
        }



        for (int i = 0; i < collisionEdgeList.Count; i++)
        {
            for (int k = 0; k < collisionBoxList.Count; k++)
            {

                edgeI = collisionEdgeList[i];
                boxK = collisionBoxList[k];

                if (PointIsInBox(edgeI.collisionCorner0, boxK))
                {
                    if (PointIsInBox(edgeI.collisionCorner1, boxK))
                    {
                        edgeI.ignoreCollision = true;
                    }
                    else if (TryGetEdgeBoxCollisionPoint(edgeI, boxK, out intersection))
                    {
                        edgeI.collisionCorner0 = intersection;
                    }
                }
                else if (PointIsInBox(edgeI.collisionCorner1, boxK))
                {
                    if (TryGetEdgeBoxCollisionPoint(edgeI, boxK, out intersection))
                    {
                        edgeI.collisionCorner1 = intersection;
                    }
                }
            }
        }
    }

    private bool PointIsInBox(Vector2 point, CollisionBox box)
    {
        return point.x >= box.left && point.x <= box.right && point.y >= box.bottom && point.y <= box.top;
    }

    private bool TryGetEdgeBoxCollisionPoint(CollisionEdge edge, CollisionBox box, out Vector2 intersection)
    {
        float minPathFraction = 1.001f;
        float curDist;
        Vector2 closestPoint = default;
        Vector2 curPoint;

        Vector2 edgeDisp = edge.collisionCorner1 - edge.collisionCorner0;

        if (edgeDisp.x != 0)
        {
            // Left Edge
            curDist = (box.left - edge.collisionCorner0.x) / edgeDisp.x;
            curPoint = edge.collisionCorner0 + edgeDisp * curDist;
            // Everything here checks for validity except the last part.
            if (curPoint.y >= box.bottom && curPoint.y <= box.top && curDist >= 0 && curDist < minPathFraction)
            {
                minPathFraction = curDist;
                closestPoint = curPoint;
            }

            // Right Edge
            curDist = (box.right - edge.collisionCorner0.x) / edgeDisp.x;
            curPoint = edge.collisionCorner0 + edgeDisp * curDist;
            if (curPoint.y >= box.bottom && curPoint.y <= box.top && curDist >= 0 && curDist < minPathFraction)
            {
                minPathFraction = curDist;
                closestPoint = curPoint;
            }
        }
        if (edgeDisp.y != 0)
        {
            // Bottom Edge
            curDist = (box.bottom - edge.collisionCorner0.y) / edgeDisp.y;
            curPoint = edge.collisionCorner0 + edgeDisp * curDist;
            if (curPoint.x >= box.left && curPoint.x <= box.right && curDist >= 0 && curDist < minPathFraction)
            {
                minPathFraction = curDist;
                closestPoint = curPoint;
            }

            // Top Edge
            curDist = (box.top - edge.collisionCorner0.y) / edgeDisp.y;
            curPoint = edge.collisionCorner0 + edgeDisp * curDist;
            if (curPoint.x >= box.left && curPoint.x <= box.right && curDist >= 0 && curDist < minPathFraction)
            {
                minPathFraction = curDist;
                closestPoint = curPoint;
            }
        }

        intersection = closestPoint;

        if (minPathFraction <= 1)
        {
            return true;
        }
        return false;
    }




    




    

    private void CreateBox(CollisionBox box)
    {
        Vector2 size = new Vector2(box.right - box.left, box.top - box.bottom);
        Vector3 center = new Vector3((box.right + box.left) / 2, (box.top + box.bottom) / 2, -0.8f);;

        GameObject boxGameObject = new()
        {
            name = "North Wall Box (" + center + ")"
        };

        boxGameObject.transform.position = center;
        boxGameObject.transform.SetParent(this.transform);

        if (drawNorthWalls)
        {
            SpriteRenderer spriteRenderer = boxGameObject.AddComponent<SpriteRenderer>();
            spriteRenderer.drawMode = SpriteDrawMode.Sliced;
            spriteRenderer.sprite = collisionSprite;
            spriteRenderer.size = size;
        }

        BoxCollider2D boxCollider2D = boxGameObject.AddComponent<BoxCollider2D>();
        boxCollider2D.size = size;

        boxGameObject.AddComponent<MapGenSpawnedObject>();
    }

    private void CreateEdge(CollisionEdge edge)
    {
        
        GameObject edgeGameObject = new()
        {
            name = "Wall Edge (" + edge.collisionCorner0 + ", "+ edge.collisionCorner1 + ")"
        };
        if (drawThinWalls)
        {
            LineRenderer lineRenderer = edgeGameObject.AddComponent<LineRenderer>();
            edgeGameObject.transform.SetParent(transform);

            lineRenderer.SetPosition(0, (Vector3)edge.visualCorner0 + new Vector3(0, 0, -0.9f));
            lineRenderer.SetPosition(1, (Vector3)edge.visualCorner1 + new Vector3(0, 0, -0.9f));
            lineRenderer.material = collisionMaterial;
            lineRenderer.startWidth = lineRenderer.endWidth = mapGenerator.tileSizeUnits / 4;
        }

        if (!edge.ignoreCollision)
        {
            EdgeCollider2D edgeCollider2D = edgeGameObject.AddComponent<EdgeCollider2D>();
            edgeCollider2D.offset = -edgeGameObject.transform.position;
            edgeCollider2D.SetPoints(new List<Vector2>
            { edge.collisionCorner0, edge.collisionCorner1 });
        }
        edgeGameObject.AddComponent<MapGenSpawnedObject>();
        
        
        // Use this code instead if you want a visually accurate representation of where the collision is.
        /*
        if (!edge.ignoreCollision)
        {
            GameObject edgeGameObject = new()
            {
                name = "Wall Edge (" + edge.collisionCorner0 + ", "+ edge.collisionCorner1 + ")"
            };
            edgeGameObject.transform.SetParent(transform);

            if (drawThinWalls)
            {
                LineRenderer lineRenderer = edgeGameObject.AddComponent<LineRenderer>();

                lineRenderer.SetPosition(0, (Vector3)edge.collisionCorner0 + new Vector3(0, 0, -0.9f));
                lineRenderer.SetPosition(1, (Vector3)edge.collisionCorner1 + new Vector3(0, 0, -0.9f));
                lineRenderer.material = collisionMaterial;
                lineRenderer.startWidth = lineRenderer.endWidth = mapGenerator.tileSizeUnits / 4;
            }

            EdgeCollider2D edgeCollider2D = edgeGameObject.AddComponent<EdgeCollider2D>();
            edgeCollider2D.offset = -edgeGameObject.transform.position;
            edgeCollider2D.SetPoints(new List<Vector2>
            { edge.collisionCorner0, edge.collisionCorner1 });

            edgeGameObject.AddComponent<MapGenSpawnedObject>();
        }
        */
        
    }
}
