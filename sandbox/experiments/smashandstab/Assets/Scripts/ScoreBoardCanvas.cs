using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using System;
using TMPro;

public class ScoreBoardCanvas : MonoBehaviour
{
    [SerializeField] Transform Grid;
    [SerializeField] GameObject playerScoreTemplete;

    private void OnEnable()
    {
        NetworkPlayer.OnPlayerSpawn += OnPlayerSpawned;
    }

    private void OnDisable()
    {
        NetworkPlayer.OnPlayerDespawn -= OnPlayerSpawned;
    }

    private void OnPlayerSpawned(GameObject player)
    {
        GameObject PlayerUI = Instantiate(playerScoreTemplete, Grid);
        PlayerUI.GetComponent<PlayerScore>().TrackPlayer(player);
    }
}
