using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Unity.Netcode;
using System;

public class NetworkPlayer : NetworkBehaviour
{
    public static event Action<GameObject> OnPlayerSpawn;
    public static event Action<GameObject> OnPlayerDespawn;

    public override void OnNetworkSpawn()
    {
        base.OnNetworkSpawn();
        OnPlayerSpawn?.Invoke(this.gameObject);
    }

    public override void OnNetworkDespawn()
    {
        base .OnNetworkDespawn();
        OnPlayerDespawn?.Invoke(this.gameObject);
    }

}
