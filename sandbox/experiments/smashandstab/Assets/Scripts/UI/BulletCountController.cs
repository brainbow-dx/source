using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using TMPro;

public class BulletCountController : MonoBehaviour
{
    public TextMeshProUGUI bulletCountText;
    private int bulletCount;

    // Start is called before the first frame update
    void Start()
    {
        bulletCountText = GetComponent<TextMeshProUGUI>();
        bulletCount = 0;
        UpdateBulletCountText();
    }

    public void UpdateBulletCount(int count){
        bulletCount = count;
        UpdateBulletCountText();
    }

    private void UpdateBulletCountText(){
        bulletCountText.text = bulletCount + "x";
    }
}
