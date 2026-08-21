using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using TMPro;

public class ObjectiveUI : MonoBehaviour
{
    [SerializeField] PlayerData data;
    [SerializeField] TMP_Text mainDesc;

    private void Update()
    {
        objective o = data.GetMainObj();

        if (o == null)
        {
            mainDesc.text = "No Main Objective";
        } else
        {
            mainDesc.text = o.desciption;
        }
    }

}
