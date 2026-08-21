using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(menuName = "Items/ItemList")]
public class ItemList : ScriptableObject
{
    public List<ItemDefinition> items;
}
