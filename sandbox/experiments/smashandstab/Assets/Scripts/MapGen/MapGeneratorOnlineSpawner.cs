using Unity.Netcode;
using UnityEngine;
using UnityEngine.SceneManagement;

public class MapGeneratorOnlineSpawner : MonoBehaviour
{
    [SerializeField]
    GameObject mapGeneratorPrefab;

    void Start()
    {
        // Destroys this if a map generator prefab is not attached.
        if (mapGeneratorPrefab == null)
        {
            Debug.Log(nameof(mapGeneratorPrefab) + "of this " + nameof(MapGeneratorOnlineSpawner) + " component is null!");
            Destroy(this.gameObject);
        }
    }

    void Update()
    {
        if (NetworkManager.Singleton.IsConnectedClient && NetworkManager.Singleton.IsListening)
        {
            // Spawn a MapGenerator only if this is the server, and destroy this object either way.
            if (NetworkManager.Singleton.IsServer)
            {
                GameObject mapGeneratorObject = Instantiate(mapGeneratorPrefab);
                SceneManager.MoveGameObjectToScene(mapGeneratorObject, gameObject.scene);
                mapGeneratorObject.GetComponent<NetworkObject>().Spawn();
            }
            Destroy(this.gameObject);
        }
    }
}
