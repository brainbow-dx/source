using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Unity.Netcode;

public class CharacterDefinition : NetworkBehaviour
{
    [SerializeField]
    public int maxHealth;
    public int health;
    [SerializeField]
    [Tooltip("Weight only has an effect up to 100.0f.")]
    public float weight;

    private void FixedUpdate() {
        die();
    }

    private void die(){
        if (health == 0){
            Destroy(this.gameObject);
        }
        else {
            return;
        }
    }

    public int GetHealth() {
        return health;
    }

    public void TakeDamage(int damage) {
        health -= damage;
        if (health < 0) {
            health = 0;
        }
    }
}
