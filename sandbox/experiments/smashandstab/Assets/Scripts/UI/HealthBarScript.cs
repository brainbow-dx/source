using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;

public class HealthBarScript : MonoBehaviour
{
    public Slider healthBar;
    public CharacterDefinition playerHealth;
    // Start is called before the first frame update
    void Start()
    {
        healthBar = GetComponent<Slider>();
        healthBar.maxValue = playerHealth.maxHealth;
        healthBar.value = playerHealth.maxHealth;
    }

    public void UpdateHealth(){
        healthBar.value = playerHealth.health;
    }
}

// using System.Collections;
// using System.Collections.Generic;
// using UnityEngine;
// using UnityEngine.UI;
// public class HealthBar : MonoBehaviour
// {
//     public Slider healthBar;
//     public Health playerHealth;
//     private void Start()
//     {
//         playerHealth = GameObject.FindGameObjectWithTag("Player").GetComponent<Health>();
//         healthBar = GetComponent<Slider>();
//         healthBar.maxValue = playerHealth.maxHealth;
//         healthBar.value = playerHealth.maxHealth;
//     }
//     public void SetHealth(int hp)
//     {
//         healthBar.value = hp;
//     }
// }
// Code language: C# (cs)
