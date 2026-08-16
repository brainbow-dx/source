using System.Collections;
using System.Collections.Generic;
using Unity.Netcode;
using UnityEngine;

public class Crate : MonoBehaviour
{
    [SerializeField]
    NetworkObject networkObject;
    [SerializeField]
    BoxCollider2D triggerBox;

    private void OnTriggerStay2D(Collider2D other) {
        if (NetworkManager.Singleton.IsServer && this != null)
            networkObject.Despawn();
    }
}
