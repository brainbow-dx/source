using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class Inventory : MonoBehaviour
{
    //create an array of ints 
    //when collecting an object run a for loop and add obtain the idnumber from the object and store it in the array
    //if inventory needs to be accessed first check if weapon else use item dictionary? pass idnumber into weapons dictionary to retrieve the gameobject
    //if i need to drop simply delete that space in the array 
    //last part of the array needs to be for bullets only

    //need to have a weight adding function whenever there is a collision with a collectible 

    int[] playerInventory = new int [6];

    [SerializeField]
    private UIController uIController;
    [SerializeField]
    private BulletCountController bulletCountController;

    [SerializeField]
    private ItemList itemList;
    [SerializeField]
    private Shooting rangedScript;
    [SerializeField]
    private InsideRange meleeScript;

    [SerializeField]
    private GameObject collectibleGameObject;
    [SerializeField]
    private PlayerController player;

    [SerializeField]
    private float spawnDistance;
    private Vector3 offsetVector;
    private float x;
    private float y;

    public int currId;
    private int currSlot;
    public int bulletCount;

    public Dictionary<int, ItemDefinition> itemListById = new Dictionary<int, ItemDefinition>();
    // public Dictionary<int, CollectibleDefinition> collectibleListById = new Dictionary<int,CollectibleDefinition>();

    void Start(){
        bulletCount = 0;
        currId = 0;
        offsetVector = new Vector3(0, 0, 0);

        ParseItemList();
        SwitchToEmpty();
    }

    public void SwitchInInventory(int slot){
        //if slot is empty return
        if (playerInventory[slot] == 0){
            return;
        }

        currId = playerInventory[slot];
        currSlot = slot;

        uIController.UpdateHighlight(slot);

        ItemDefinition currItem = itemListById[currId];
        MeleeWeaponDefinition meleeWeapon = currItem as MeleeWeaponDefinition;
        RangedWeaponDefinition rangedWeapon = currItem as RangedWeaponDefinition;
        // GameObject meleeGameObject = meleeWeapon.transform.gameObject; 
        GameObject rangedGameObject = rangedScript.gameObject; 
        // Check if currItem is MeleeWeaponDefinition
        if (meleeWeapon != null) {
            meleeScript.UpdateWeaponData(meleeWeapon);
            // meleeGameObject.SetActive(true);
            rangedGameObject.SetActive(false);
            return; // Exit the method if it's a melee weapon
        }

        // Check if currItem is RangedWeaponDefinition
        
        if (rangedWeapon != null) {
            rangedScript.UpdateWeaponData(rangedWeapon);
            rangedGameObject.SetActive(true);
            return; // Exit the method if it's a ranged weapon
        }

        //write in what happens if it is a non-weapon item
    }

    public void SwitchToEmpty(){
        meleeScript.UpdateWeaponData(null);
        rangedScript.UpdateWeaponData(null);
        currId = 0;
        uIController.UpdateHighlight();
    }


    //this version is for non-melee weapon items
    public void AddItem(int id){
        for (int i = 0; i < playerInventory.Length - 1; i++ /*Preventing access to the bullet slot*/)
        {
            if (id == 99){ // 99 as a temp placeholder for bullet
                bulletCount++;
                playerInventory[5] = bulletCount;
                bulletCountController.UpdateBulletCount(bulletCount);

                // player.TakeDamage(2);
                // Debug.Log("Bullet count: " + playerInventory[5]);
                return;
            }
            if (playerInventory[i] == 0){ // Assuming 0 means empty slot
                playerInventory[i] = id; // Place the item id in the first empty slot found
                SwitchInInventory(i); //switches to added item
                uIController.UpdateInventoryImage(i, itemListById[id].importedSprite);
                player.AdjustWeight(id, true);

                // No durability is stored for this version
                Debug.Log("Item added to inventory at index " + i);
                PrintInventory(); // Optional: Print the updated inventory
                return;
            }
        }
        Debug.Log("Inventory is full. Cannot add item.");
        PrintInventory();
    }

    public void SubtractBullet(){
        bulletCount--;
        playerInventory[5]--;
        bulletCountController.UpdateBulletCount(bulletCount);
        PrintInventory();
    }

    public void SpawnDroppedItem(int id){
        SetCollectibleData(itemListById[id]);
        if (collectibleGameObject != null){
           
            // Calculate spawn position based on player's last movement direction
            Vector3 playerDirection = player.lastMovementDirection.normalized;
            Vector3 playerPosition = player.transform.position;
            Vector3 spawnOffset = CalculateOffSetVector(playerPosition, playerDirection);

            GameObject collectible = Instantiate(collectibleGameObject, player.transform.position + spawnOffset, Quaternion.identity);
            RemoveItem(id);        
        }
        else {
            Debug.LogWarning("No item in this inventory slot!");
        }
    }

    public void RemoveItem(int id){
        for (int i = 0; i < playerInventory.Length - 1; i++){
            if (playerInventory[i] == id){
                player.AdjustWeight(id, false);
                playerInventory[i] = 0; 
                uIController.UpdateInventoryImage(i);
                break;
            }
        }
        SwitchToEmpty();
        PrintInventory();
    }

    private Vector3 CalculateOffSetVector(Vector3 playerPos, Vector3 playerDir){
        x = playerDir.x;
        offsetVector = Vector3.zero;
        
        //shoot a raycast behind the player to see if there is a wall in the way
        Vector3 raycastOrigin = playerPos - playerDir * spawnDistance;
        Vector3 raycastDirection = -playerDir;
        RaycastHit2D hit = Physics2D.Raycast(raycastOrigin, raycastDirection, spawnDistance);

        if(hit.collider != null && hit.collider.CompareTag("Wall")){
            return offsetVector;
        }


        if (x != 0){
            if (x > 0){
                offsetVector.x = -spawnDistance;
            }
            else {
                offsetVector.x = spawnDistance;
            }
        }
        return offsetVector + playerDir;
    }

    //adds all items into item list
    private void ParseItemList(){
        foreach (ItemDefinition item in itemList.items){
            if(!itemListById.ContainsKey(item.idNumber)){
                itemListById.Add(item.idNumber, item);
                Debug.Log("Added item to dictionary: " + item.idNumber);
            }
            else {
                Debug.LogWarning("Duplicate item ID item.idNumber " + item.idNumber);
            }
        }
        PrintItemList(itemListById);
    }

    //helper print methods
    private void PrintInventory(){
        for (int i = 0; i < playerInventory.Length; i++){
            Debug.Log("Slot " + i + ": " + playerInventory[i]);
        }
    }

    private void PrintItemList(Dictionary<int, ItemDefinition> dictionary){
        foreach (KeyValuePair<int, ItemDefinition> kvp in dictionary){
            Debug.Log("Key = " + kvp.Key + ", ItemDefinition Name = " + kvp.Value);
        }
    }

    private void PrintDurabilities(Dictionary<int, int> dictionary){
        foreach (KeyValuePair<int, int> kvp in dictionary){
            Debug.Log("ID = " + kvp.Key + ", Durability = " + kvp.Value);
        }
    }

    private void SetCollectibleData(ItemDefinition newCollectible){
        CollectibleScript collectScript = collectibleGameObject.GetComponent<CollectibleScript>();
        collectScript.itemDef = newCollectible;
        collectScript.InitializeCollectible();
    }

    //helper get and check methods
    public bool IsEmpty(int slot){
        if (playerInventory[slot] == 0){
            return true;
        }
        return false;
    }

    public int GetSlot(int slot){
        return playerInventory[slot];
    }

    public int GetBulletCount(){
        return bulletCount;
    }

}
