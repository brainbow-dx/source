using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;
using Unity.Netcode;
using Cinemachine;
using UnityEngine.EventSystems;

public class NetworkManagerUI : NetworkBehaviour
{
    [SerializeField] private Button hostBtn;
    [SerializeField] private Button clientBtn;
    [SerializeField] private Button startBtn;
    [SerializeField] private ProjectSceneManager scene_changer;
    [SerializeField] private Canvas canvas;
    [SerializeField] private EventSystem es;

    private void Awake() {
        es.enabled = true;
        hostBtn.onClick.AddListener(() =>
        {
            NetworkManager.Singleton.StartHost();
            startBtn.gameObject.SetActive(true);
            hostBtn.gameObject.SetActive(false);
            clientBtn.gameObject.SetActive(false);
        });
        clientBtn.onClick.AddListener(() =>
        {
            NetworkManager.Singleton.StartClient();
            hostBtn.gameObject.SetActive(false);
            clientBtn.gameObject.SetActive(false);
        });
        startBtn.onClick.AddListener(() =>
        {
            scene_changer.toMain();
            
            canvas.enabled = false;
        });
    }
}
