using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;

public class UIController : MonoBehaviour
{
    public Canvas canvas; 
 
    public Image[] slots;
    public Image[] highlights;
    // Start is called before the first frame update
    void Start(){
        Transform canvasTransform = canvas.transform;
        Transform highlightTransform = canvasTransform.Find("Highlights");
        
        slots = new Image[5];
        highlights = new Image[5];

        slots[0] = transform.Find("Slot1").GetComponent<Image>();
        slots[1] = transform.Find("Slot2").GetComponent<Image>();
        slots[2] = transform.Find("Slot3").GetComponent<Image>();
        slots[3] = transform.Find("Slot4").GetComponent<Image>();
        slots[4] = transform.Find("Slot5").GetComponent<Image>();

        highlights[0] = highlightTransform.Find("Highlight1").GetComponent<Image>();
        highlights[1] = highlightTransform.Find("Highlight2").GetComponent<Image>();
        highlights[2] = highlightTransform.Find("Highlight3").GetComponent<Image>();
        highlights[3] = highlightTransform.Find("Highlight4").GetComponent<Image>();
        highlights[4] = highlightTransform.Find("Highlight5").GetComponent<Image>();

        for (int i = 0; i < highlights.Length; i++){
            highlights[i].enabled = false;
        }
    }

    // // Update is called once per frame
    // void Update()
    // {
        
    // }

    //used when removing inventory
    public void UpdateInventoryImage(int slot){
        Debug.Log("slot val: " + slot);
        slots[slot].sprite = null;
    }

    public void UpdateInventoryImage(int slot, Sprite newSprite){
        Debug.Log("slot val: " + slot);
        slots[slot].sprite = newSprite;
    }

    public void UpdateHighlight(){
        for (int i = 0; i < highlights.Length; i++){
            highlights[i].enabled = false;
        }
    }

    public void UpdateHighlight(int slot){
        for (int i = 0; i < highlights.Length; i++){
            highlights[i].enabled = false;
        }
        highlights[slot].enabled = true;
    }
}
