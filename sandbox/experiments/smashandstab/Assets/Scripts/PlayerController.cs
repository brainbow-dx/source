using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.AI;
using Cinemachine;

public class PlayerController : CharacterDefinition{
    [SerializeField]
    private float _baseSpeed = 4.0f;
    private float _currentSpeed;
    Animator _animator;
    Rigidbody2D rb;
    [SerializeField] private CinemachineVirtualCamera vc;
    [SerializeField] private AudioListener listener;

    [SerializeField]
    [Tooltip("Enter this number as 50 for 50%.")]
    private float minSpeedReduction;
    private float maxSpeedReduction; //always set to 1.0f so that number is clamped to 100% of speed
    private float weightFactor;

    [SerializeField]
    private Inventory inventory;
    
    //left is true right is false (char is default facing left)
    //tracks the the direction that the player last went
    //default is left
    //fix this by changing how it checks for left depending on player rotation
    public bool lastDirection = true; 

    public Vector2 lastMovementDirection;

    public bool isOverlapping = false;
    private GameObject overlappingGameObject;

    private int id;
    
    // Start is called before the first frame update
    void Start(){
        // inventory = GetComponentInChildren<Inventory>();
        _animator = GetComponentInChildren<Animator>();
        
        //freezes the rotation of the gameobject NOTE: this freezes all axes 
        rb = GetComponent<Rigidbody2D>();
        rb.freezeRotation = true;

        minSpeedReduction = minSpeedReduction / 100.0f;
        maxSpeedReduction = 1.0f;
        //set base weight for characters
        AdjustSpeed();
    }

    public override void OnNetworkSpawn()
    {
        transform.position = GameObject.FindGameObjectWithTag("lobby").transform.position;
        if (IsOwner)
        {
            listener.enabled = true;
            vc.Priority = 1;
        } else
        {
            vc.Priority = 0;
        }
    }


    // Update is called once per frame
    void Update(){
        if (!IsOwner) return;

         //moves the player
        transform.Translate(SaveDirectionAndSpeed() * _currentSpeed * Time.deltaTime);
        CheckInventoryInput();        
    }

    //Adjust the speed based on weight
    private void AdjustSpeed(){
        //For example:
            // If weight is 0, this results in 1.0 / 1.0 = 1.0.
            // If weight is 50, this results in 1.0 / 1.5 ≈ 0.67.
            // If weight is 100, this results in 1.0 / 2.0 = 0.5.
        weightFactor = Mathf.Clamp(1.0f / (1.0f + (weight / 100.0f)), minSpeedReduction, maxSpeedReduction);
        _currentSpeed = _baseSpeed * weightFactor;
    }

    //pick up items
    private void OnTriggerEnter2D(Collider2D other) {
        GameObject otherGameObject = other.gameObject;
        if (otherGameObject.CompareTag("Collectible")) {
            // Get the collectible definition component
            CollectibleScript collectible = otherGameObject.GetComponent<CollectibleScript>();
            ItemDefinition collectibleDef = collectible.itemDef;

            if (collectibleDef != null) {
                id = collectibleDef.idNumber;
                float itemWeight = collectibleDef.weight;
                if(id == 99){
                    Destroy(otherGameObject); 
                    inventory.AddItem(id);
                    return;
                }
                isOverlapping = true;
                overlappingGameObject = otherGameObject;
            }
        }
    }

    private void OnTriggerExit2D(Collider2D other){
        if(other.gameObject.CompareTag("Collectible")) {
            overlappingGameObject = null;
        }
    }
    
    public void AdjustWeight(int id, bool add){
        if (add == true){
            float itemWeight = inventory.itemListById[id].weight;
            weight += itemWeight;
            Debug.Log("Weight added: " + itemWeight);
        }
        else {
            float itemWeight = inventory.itemListById[id].weight;
            weight -= itemWeight;
            Debug.Log("Weight dropped: " + itemWeight);
        }
        AdjustSpeed();
    }

    //updates the direction that the character is facing and if they are moving or not
    //returns vector that player is going
    private Vector3 SaveDirectionAndSpeed(){
        float hi = Input.GetAxis("Horizontal");
        float vi = Input.GetAxis("Vertical");
        Vector3 d = new Vector3(hi, vi, 0).normalized;

        if (hi != 0 || vi != 0){
            lastMovementDirection = d;
        } 

        //this is for the direction the character faces
        if (hi != 0)
        {
            if (hi > 0)
            {
                transform.localScale = new Vector3(-1, 1, 1);
                lastDirection = false;;
            }
            else
            {
                transform.localScale = new Vector3(1, 1, 1);
                lastDirection = true;
            }
        }

        if (vi != 0 || hi != 0)
        {
            _animator.SetFloat("speed", _currentSpeed);
        } else
        {
            _animator.SetFloat("speed", 0f);
        }

        return d;
    }

    private void CheckInventoryInput(){
        if (Input.GetKeyDown(KeyCode.Alpha1)){
            inventory.SwitchInInventory(0);
        }
        else if (Input.GetKeyDown(KeyCode.Alpha2)){
            inventory.SwitchInInventory(1);
        }
        else if (Input.GetKeyDown(KeyCode.Alpha3)){
            inventory.SwitchInInventory(2);
        }
        else if (Input.GetKeyDown(KeyCode.Alpha4)){
            inventory.SwitchInInventory(3);
        }
        else if (Input.GetKeyDown(KeyCode.Alpha5)){
            inventory.SwitchInInventory(4);
        }
        else if(Input.GetKeyDown(KeyCode.Alpha0)){
            inventory.SwitchToEmpty();
        }
        else if (Input.GetKeyDown(KeyCode.Q)){
            //drop currently selected inventory
            inventory.SpawnDroppedItem(inventory.currId);
        }
        else if (Input.GetKeyDown(KeyCode.E)){
            if(overlappingGameObject != null){
                // Destroy the collectible object
                Destroy(overlappingGameObject); 
                // Add item to inventory
                inventory.AddItem(id);
            }

        }
        else{
            return;
        }
    }
}
