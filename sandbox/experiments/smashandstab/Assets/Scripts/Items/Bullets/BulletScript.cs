using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class BulletScript : MonoBehaviour
{
    [SerializeField]
    public BulletDefinition bd;

    private SpriteRenderer spriteRenderer;
    private Sprite importedSprite;
    private BoxCollider2D boxCollider;

    //normal bullet variables
    private Vector3 mousePos;
    private Camera mainCam;
    [Tooltip("No need to serialize this.")]
    public Rigidbody2D rb;
    private float force;

    public bool firstStruck;

    private int damage;

    private RangedWeaponDefinition gunData;

    protected void Start(){  
        spriteRenderer = GetComponent<SpriteRenderer>();
        boxCollider = GetComponent<BoxCollider2D>();
        importedSprite = bd.importedSprite;
        LoadSprite(importedSprite);
        UpdateBoxColliderSize();

        //calculates what direction to shoot the bullet and applies force to it 
        rb = GetComponent<Rigidbody2D>();
        force = bd.force;

        Vector3 direction = mousePos - transform.position;
        rb.linearVelocity = new Vector2(direction.x, direction.y).normalized * force;
        float rot = Mathf.Atan2(direction.y, direction.x) * Mathf.Rad2Deg;
        transform.rotation = Quaternion.Euler(0, 0, rot + 180);

        damage = gunData.damage;
        //self destruct after x seconds
        Destroy(gameObject, 8.0f);
    }

    protected virtual void OnCollisionEnter2D(Collision2D other){
        GameObject otherObject = other.gameObject;
        if (otherObject.CompareTag("Wall")){
            Destroy(this.gameObject);
        }
        else {
            CharacterDefinition character = otherObject.GetComponentInParent<CharacterDefinition>();
            if (character != null){
                character.TakeDamage(damage); // Apply damage to the character
                // hasDealtDamage = true;
                Debug.Log("Raycast hit: " + otherObject.name + " Health: " + character.GetHealth());
            }
            Destroy(this.gameObject);
        }
    }

    public void SetMousePos(Vector3 mp){
        mousePos = mp;
    }

    public void SetGunData(RangedWeaponDefinition data)
    {
        gunData = data;
    }

    private void LoadSprite(Sprite newSprite){
        if (newSprite != null){
            spriteRenderer.sprite = newSprite;
        }
        else {
            Debug.LogWarning("Sprite not found: " + name);
        }
    }

    private void UpdateBoxColliderSize(){
        if (spriteRenderer.sprite != null){
            Vector2 spriteSize = spriteRenderer.sprite.bounds.size;
            boxCollider.size = spriteSize;
        }
        else {
            Debug.LogWarning("Sprite is null. Cannot adjust BoxCollider size.");
        }
    }
}
