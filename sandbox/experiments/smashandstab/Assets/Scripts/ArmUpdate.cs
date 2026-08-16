using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Live2D.Cubism.Core;
using Unity.Netcode;

public class Arm : NetworkBehaviour
{
    /// <summary>
    /// This script controls the animation of Live2D Model. It has no impact on shooting. Ask jarett if you are looking for that script
    /// </summary>
    [SerializeField]
    private Camera mainCam;
    private Vector3 mousePos;
    [SerializeField]
    CubismModel model;
    [SerializeField]
    Animator animator;
    [SerializeField]
    private Transform scale;

    private NetworkVariable<float> dir = new NetworkVariable<float>(1.0f, NetworkVariableReadPermission.Everyone, NetworkVariableWritePermission.Owner);
    private NetworkVariable<float> rotZ = new NetworkVariable<float>(0.0f, NetworkVariableReadPermission.Everyone, NetworkVariableWritePermission.Owner);


    // These give names to the live2d param which are just numbers
    [SerializeField]
    private int head;

    [SerializeField]
    private int armRot;

    [SerializeField]
    private int norArmVis;

    [SerializeField]
    private int revArmVis;


    private void Update()
    {
        if (!IsOwner) return;
        dir.Value = scale.localScale.x;
        mousePos = mainCam.ScreenToWorldPoint(Input.mousePosition);
        Vector3 rotation = mousePos - transform.position;
        rotZ.Value = Mathf.Atan2(rotation.y, rotation.x) * Mathf.Rad2Deg;
    }

    // Update is called once per frame
    void LateUpdate()
    {
        // A garbage amount of math to map the unity rotation system to the live2D one
        float localdir = dir.Value;

        if (localdir > 0)
        {
            localdir = 1;
        }
        else
        {
            localdir = -1;
        }

        float mapped = (rotZ.Value * -1) - 90;

        if (mapped < -180)
        {
            mapped += 360;
        }


        // check if we are shooting before we do anything. 

        if (animator.GetBool("shooting")){
            //turn off the redudant arm
            model.Parameters[16].Value = 0f;
            if (localdir > 0)
            {
                if (mapped > 0)
                {
                    model.Parameters[head].Value = 1f;
                    model.Parameters[norArmVis].Value = 1f;
                    model.Parameters[revArmVis].Value = 0f;
                    model.Parameters[armRot].Value = mapped * localdir;

                }
                else
                {
                    model.Parameters[head].Value = 0f;
                    model.Parameters[norArmVis].Value = 0f;
                    model.Parameters[revArmVis].Value = 1f;
                    model.Parameters[armRot].Value = mapped * localdir * -1;
                }
            }
            else
            {
                if (mapped > 0)
                {
                    model.Parameters[head].Value = 0f;
                    model.Parameters[norArmVis].Value = 0f;
                    model.Parameters[revArmVis].Value = 1f;
                    model.Parameters[armRot].Value = mapped * localdir * -1;
                }
                else
                {
                    model.Parameters[head].Value = 1f;
                    model.Parameters[norArmVis].Value = 1f;
                    model.Parameters[revArmVis].Value = 0f;
                    model.Parameters[armRot].Value = mapped * localdir;
                }
            }
        } else
        {
            model.Parameters[norArmVis].Value = 0f;
            model.Parameters[revArmVis].Value = 0f;
            model.Parameters[16].Value = 1f;
        }

        //Head still follows the mouse
        if (localdir > 0)
        {
            if (mapped > 0)
            {
                model.Parameters[head].Value = 1f;
            }
            else
            {
                model.Parameters[head].Value = 0f;
            }
        }
        else
        {
            if (mapped > 0)
            {
                model.Parameters[head].Value = 0f;
            }
            else
            {
                model.Parameters[head].Value = 1f;
            }
        }
    }
}