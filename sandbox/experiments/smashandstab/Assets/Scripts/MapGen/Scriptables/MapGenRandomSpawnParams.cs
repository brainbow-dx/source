using System;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(fileName = nameof(MapGenRandomSpawnParams), menuName = "MapGen/" + nameof(MapGenRandomSpawnParams))]
public class MapGenRandomSpawnParams : ScriptableObject
{
    // Self-explanatory. Exists only to pair with the corresponding list.
    [Serializable]
    public class WeightedObject
    {
        public GameObject theObject;
        public float weight;
    }

    // A list of weighted objects. The higher their weight, the more often they spawn. A null slot means no object, and is also weighted.
    [SerializeField]
    public List<WeightedObject> spawnableWeightedObjects;
}
