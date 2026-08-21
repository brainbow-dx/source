using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(menuName = "Weapons/RangedWeapon_Definitions")]
public class RangedWeaponDefinition : WeaponDefinition {
    [SerializeField]
    public float _reloadRate;
    [SerializeField]
    public GameObject bulletObject;
    // [SerializeField]
    // public int ammoSize;
}