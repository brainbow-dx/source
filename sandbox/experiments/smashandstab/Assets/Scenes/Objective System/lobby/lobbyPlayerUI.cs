using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using TMPro;

public class lobbyPlayerUI : MonoBehaviour
{
    [SerializeField]
    private TMP_Text text;


    public void intialize(string name)
    {
        text.text = name;
    }

}
