using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;
using Unity.Netcode;

public class Main_UI : NetworkBehaviour
{
    [SerializeField] private Button objBtn;
    [SerializeField] private Button endRoundBtn;
    private Canvas canvas;

    private void Awake()
    {
        canvas = GameObject.FindGameObjectWithTag("LobbyCanvas").GetComponent<Canvas>();
        endRoundBtn.onClick.AddListener(() =>
        {
            ProjectSceneManager ps = GameObject.FindGameObjectWithTag("psmanager").GetComponent<ProjectSceneManager>();
            GameObject lobby = GameObject.FindGameObjectWithTag("lobby");
            GameObject.FindGameObjectWithTag("teleport").GetComponent<PlayerTeleport>().TeleportAllPlayers();
            GameObject.FindWithTag("objManager").GetComponent<ObjectiveManager>().ClearMainObjAllPlayers();
            canvas.enabled = true;
            ps.UnloadScene();

            GameObject mapGenObject = GameObject.FindWithTag("MapGenerator");
            mapGenObject.GetComponent<MapGenerator>().DespawnMapGeneratorSpawnedObjects();
            mapGenObject.GetComponent<NetworkObject>().Despawn();
        });

        objBtn.onClick.AddListener(() =>
        {
            GameObject.FindWithTag("objManager").GetComponent<ObjectiveManager>().AddMainObjAllPlayers();
        });
    }

}
