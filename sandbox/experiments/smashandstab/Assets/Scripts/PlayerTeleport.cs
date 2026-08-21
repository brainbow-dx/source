using Unity.Netcode;
using UnityEngine;

public class PlayerTeleport : NetworkBehaviour
{
    [SerializeField]
    private Transform teleportTarget; // Assign the teleport target in the Inspector

    // Call this method on the server to teleport all players
    public void TeleportAllPlayers()
    {
        if (IsServer)
        {
            foreach (var networkObject in NetworkManager.Singleton.ConnectedClientsList)
            {
                var player = networkObject.PlayerObject;
                if (player != null)
                {
                    TeleportPlayerServerRpc(player.NetworkObjectId, teleportTarget.position, teleportTarget.rotation);
                }
            }
        }
    }

    [ServerRpc]
    private void TeleportPlayerServerRpc(ulong playerId, Vector3 position, Quaternion rotation)
    {
        var networkObject = NetworkManager.Singleton.SpawnManager.SpawnedObjects[playerId];
        if (networkObject != null)
        {
            networkObject.transform.position = position;
            networkObject.transform.rotation = rotation;

            // Optionally, you can also notify the clients
            TeleportPlayerClientRpc(networkObject.NetworkObjectId, position, rotation);
        }
    }

    [ClientRpc]
    private void TeleportPlayerClientRpc(ulong playerId, Vector3 position, Quaternion rotation)
    {
        var networkObject = NetworkManager.Singleton.SpawnManager.SpawnedObjects[playerId];
        if (networkObject != null && networkObject.IsOwner)
        {
            networkObject.transform.position = position;
            networkObject.transform.rotation = rotation;
        }
    }
}


