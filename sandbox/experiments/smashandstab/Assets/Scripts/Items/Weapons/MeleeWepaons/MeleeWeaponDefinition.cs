using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(menuName = "Weapons/MeleeWeapon_Definitions")]
public class MeleeWeaponDefinition : WeaponDefinition{
    [SerializeField]
    public int durability;
    [SerializeField]
    public float length;
}
