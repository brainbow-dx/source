using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(menuName = "Items/Item_Definitions")]
public class ItemDefinition : ScriptableObject
{
    public int idNumber;
    public int reward;
    public string description;
    public float weight;
    public Sprite importedSprite;
}
