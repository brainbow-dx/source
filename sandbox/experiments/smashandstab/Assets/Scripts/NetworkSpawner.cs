using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;
using Unity.Netcode;

public class NetworkSpawner : NetworkBehaviour
{
    // [SerializeField] private NetworkObject lobbyAmmo;
    
    private NetworkManager networkManager;
    // Start is called before the first frame update
    void Start()
    {
        networkManager = FindObjectOfType<NetworkManager>();

        // if (IsServer){
        //     SpawnAmmo();
        // }
    }

    // private void SpawnAmmo(){
    //     //Access the NetworkPrefabList from the NetworkManager
    //     NetworkObject lobbyAmmoPrefab = networkManager.NetworkConfig.NetworkPrefabs.Find(prefab => prefab.Prefab.name == "AmmoCollectible").Prefab.GetComponent<NetworkObject>();



    //     if (lobbyAmmoPrefab != null){
    //         NetworkObject instance = Instantiate(lobbyAmmoPrefab);
    //         instance.Spawn();
    //     }
    //     else {
    //         Debug.LogError("Prefab not found in the NetworkPrefabList.");
    //     }
    // }
}
