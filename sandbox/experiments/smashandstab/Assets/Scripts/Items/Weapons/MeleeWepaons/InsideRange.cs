using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Unity.Netcode;

public class InsideRange : NetworkBehaviour
{
    public List<GameObject> inRangeList;

    private bool hasDealtDamage;
    // private InsideRange insideRange;
    private GameObject target;
    private RaycastHit2D raycastHit;
    [SerializeField]
    private Inventory playerInventory;

    private MeleeWeaponDefinition weaponData;
    private float length;
    private int damage;
    private int durability;

    private CircleCollider2D collider;

    [SerializeField]
    private Camera camera;

    void Start(){
        hasDealtDamage = false;
        // insideRange = transform.parent.GetComponent<InsideRange>();

        // Step 1: Find the WeaponController GameObject (parent's sibling)
        // GameObject weaponControllerObject = transform.parent.parent.gameObject;
        // Step 2: Find the Inventory component on the Player (sibling to WeaponController)
        // playerInventory = weaponControllerObject.GetComponentInParent<PlayerController>().GetComponentInChildren<Inventory>();
    }

    // Update is called once per frame
    void Update(){
        if (!IsOwner) return;
        Vector2 ray = camera.ScreenToWorldPoint(Input.mousePosition);
    
        // Perform raycast
        raycastHit = Physics2D.Raycast(ray, Vector2.zero);
        
        // Check if the raycast hits something
        if (raycastHit.collider != null){
            
            if (Input.GetMouseButtonDown(0) && !hasDealtDamage){ /*&& !EventSystem.current.IsPointerOverGameObject() && raycastHit.transform.gameObject.tag == "Enemy")*/
                target = raycastHit.transform.gameObject;
                CheckRayCastHit();
            }
        }

        // Reset the flag when the left mouse button is released
        if (Input.GetMouseButtonUp(0))
        {
            hasDealtDamage = false;
        }
    }

    //checks if raycasted object is inside range
    private bool CheckList(GameObject clicked){
        // List insideRange<GameObject>() = InsideRange.inRangeList;
        if(inRangeList.Count < 0){
            return false;
        }
        else if(inRangeList.Contains(clicked)){
            return true;
        }
        else {
            return false;
        }
    }

    //check if raycast hit is in range, deal damage, check durability
    private void CheckRayCastHit(){
        //if not in range do not register raycast hit    
        if(CheckList(raycastHit.transform.gameObject) == false){ 
            Debug.Log("Out of range");
            return;
        } else {
            target = raycastHit.transform.gameObject;

            // Check if the target has a CharacterDefinition component
            CharacterDefinition character = target.GetComponentInParent<CharacterDefinition>();
            if (character != null){
                // Apply damage to the character
                character.TakeDamage(damage); 
                hasDealtDamage = true;
                durability--;
                Debug.Log("Raycast hit: " + target.name + " Health: " + character.GetHealth());
            }
            //remove weapon from inventory
            if (durability <= 0){
                playerInventory.RemoveItem(weaponData.idNumber);
            }
        }
    }

    private void OnTriggerEnter2D(Collider2D collider){
        if(!inRangeList.Contains(collider.gameObject) && collider.gameObject.tag == "Enemy"){
            inRangeList.Add(collider.gameObject);
            
            Debug.Log("Added " + collider.gameObject.name);
        }
    }

    private void OnTriggerExit2D(Collider2D collider){
        if(inRangeList.Contains(collider.gameObject)){
            inRangeList.Remove(collider.gameObject);

            Debug.Log("Removed " + collider.gameObject.name);
        }
    }

    public void UpdateWeaponData(MeleeWeaponDefinition newWeaponData){
        weaponData = newWeaponData;
        if (weaponData != null){
            durability = weaponData.durability;
            length = weaponData.length;
            damage = weaponData.damage;
            //set size of detector circle
            collider = GetComponent<CircleCollider2D>();
            collider.radius = length;
        }
        else {
            durability = 0;
            length = 0;
            damage = 0;
            //set size of detector circle
            collider = GetComponent<CircleCollider2D>();
            collider.radius = 0;
        }

    }
}
