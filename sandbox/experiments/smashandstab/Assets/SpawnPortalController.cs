using Unity.Netcode;
using UnityEngine;

namespace Aby
{
    public class SpawnPortalController : NetworkBehaviour
    {
        [SerializeField] private GameObject enemyPrefab;
        [SerializeField] private float spawnInterval = 20f;
        [SerializeField] private bool isGatewayOpen = true;
        [SerializeField] private int maxEntities = 1;

        private int spawnCount;
        private float timer;

        void Start()
        {
            timer = spawnInterval;
            spawnCount = 0;
        }

        void Update()
        {
            if (!IsServer || !isGatewayOpen) return;

            timer -= Time.deltaTime;
            if (timer <= 0f)
            {
                SpawnEntity();
                timer = spawnInterval;
            }
        }

        void SpawnEntity()
        {
            if (enemyPrefab == null || spawnCount >= maxEntities) return;

            var instance = Instantiate(enemyPrefab, transform.position, Quaternion.identity);
            var networkObject = instance.GetComponent<NetworkObject>();
            if (networkObject == null)
            {
                Debug.LogWarning("SpawnPortalController: enemyPrefab has no NetworkObject component.");
                Destroy(instance);
                return;
            }

            networkObject.Spawn();
            instance.layer = LayerMask.NameToLayer("Enemies");
            instance.tag = "Enemy";
            spawnCount++;
        }
    }
}
