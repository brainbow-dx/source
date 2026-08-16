using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class scoreBoard : MonoBehaviour
{
    [SerializeField] private GameObject GameMenu;
    // Update is called once per frame
    void Update()
    {
        if (Input.GetKeyDown(KeyCode.Tab))
        {
            GameMenu.SetActive(!GameMenu.activeSelf);
        }

    }
}
