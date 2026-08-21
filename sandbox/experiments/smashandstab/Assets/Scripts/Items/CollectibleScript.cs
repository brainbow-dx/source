using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class CollectibleScript : MonoBehaviour
{
    [SerializeField]
    public ItemDefinition itemDef;
   
    private SpriteRenderer spriteRenderer;
    private Sprite importedSprite;
    private BoxCollider2D boxCollider;

    void Awake(){
        spriteRenderer = GetComponent<SpriteRenderer>();
        boxCollider = GetComponent<BoxCollider2D>();
        Debug.Log("awake");
    }

    void Start(){
        importedSprite = itemDef.importedSprite;
        LoadSprite(importedSprite);
        UpdateBoxColliderSize();
    }

    public void LoadSprite(Sprite newSprite){
        if (newSprite != null){
            spriteRenderer.sprite = newSprite;
        }
        else {
            Debug.LogWarning("Sprite not found: " + name);
        }
    }

    public void UpdateBoxColliderSize(){
        if (spriteRenderer.sprite != null){
            Vector2 spriteSize = spriteRenderer.sprite.bounds.size;
            boxCollider.size = spriteSize;
        }
        else {
            Debug.LogWarning("Sprite is null. Cannot adjust BoxCollider size.");
        }
    }

    public void InitializeCollectible()
    {
        spriteRenderer = GetComponent<SpriteRenderer>();
        boxCollider = GetComponent<BoxCollider2D>();
        if (itemDef != null)
        {
            importedSprite = itemDef.importedSprite;
            LoadSprite(importedSprite);
            UpdateBoxColliderSize();
        }
        else
        {
            Debug.LogWarning("CollectibleDefinition is null. Cannot initialize collectible.");
        }
    }
}
