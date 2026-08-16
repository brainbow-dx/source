using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Unity.Netcode;
using Unity.Collections;

public class PlayerData : NetworkBehaviour
{
    public NetworkVariable<int> Score = new NetworkVariable<int>(0, NetworkVariableReadPermission.Everyone, NetworkVariableWritePermission.Owner);
    public NetworkVariable<FixedString128Bytes> Name = new NetworkVariable<FixedString128Bytes>();

    private objective MainObj;

    public void AddMainObj(objective o)
    {
        MainObj = o;
    }

    public void ClearMainObj()
    {
        MainObj = null;
    }

    public objective GetMainObj()
    {
        return MainObj;
    }

    private void Update()
    {
        if (!IsOwner) return;

        if (Input.GetKeyDown("space"))
        {
            if (MainObj != null)
            {
                Score.Value += 1000;
                ClearMainObj();
            }
        }
    }
}
