using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu(menuName = "Bullets/Bullet_Definitions")]
public class BulletDefinition : ItemDefinition
{   
    [Tooltip("This will control the speed of the player.")]
    [SerializeField] 
    public float force;
}
