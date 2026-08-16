using UnityEngine;
using Unity.Netcode;
using System.Collections.Generic;

public class ObjectiveManager : NetworkBehaviour
{
    [SerializeField] List<objective> MainObjs;

    public void AddMainObjAllPlayers()
    {
        if (IsServer)
        {
            foreach (var networkObject in NetworkManager.Singleton.ConnectedClientsList)
            {
                var player = networkObject.PlayerObject;
                if (player != null)
                {
                    AddMainObjClientRpc(player.NetworkObjectId);
                }
            }
        }
    }

    public void ClearMainObjAllPlayers()
    {
        if (IsServer)
        {
            foreach (var networkObject in NetworkManager.Singleton.ConnectedClientsList)
            {
                var player = networkObject.PlayerObject;
                if (player != null)
                {
                    ClearMainObjClientRpc(player.NetworkObjectId);
                }
            }
        }
    }

    [ClientRpc]
    private void AddMainObjClientRpc(ulong playerId)
    {
        var networkObject = NetworkManager.Singleton.SpawnManager.SpawnedObjects[playerId];
        if (networkObject != null)
        {
            networkObject.gameObject.GetComponent<PlayerData>().AddMainObj(MainObjs[Random.Range(0, MainObjs.Count)]);
        }
    }

    [ClientRpc]
    private void ClearMainObjClientRpc(ulong playerId)
    {
        var networkObject = NetworkManager.Singleton.SpawnManager.SpawnedObjects[playerId];
        if (networkObject != null)
        {
            networkObject.gameObject.GetComponent<PlayerData>().ClearMainObj();
        }
    }


}



