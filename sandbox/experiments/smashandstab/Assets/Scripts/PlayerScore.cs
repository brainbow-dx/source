using Unity.Netcode;
using Unity.Collections;
using System;
using UnityEngine;
using TMPro;
public class PlayerScore : MonoBehaviour
{
    [SerializeField] TMP_Text NameUI;
    [SerializeField] TMP_Text ScoreUI;
    
    public void TrackPlayer(GameObject player)
    {
        player.GetComponent<PlayerData>().Name.OnValueChanged += OnNameChanged;
        player.GetComponent<PlayerData>().Score.OnValueChanged += OnScoreChanged;
        OnScoreChanged(0, player.GetComponent<PlayerData>().Score.Value);
        OnNameChanged("", player.GetComponent <PlayerData>().Name.Value);

    }

    private void OnNameChanged(FixedString128Bytes previousValue, FixedString128Bytes newValue)
    {
        NameUI.text = newValue.ToString(); 
    }

    private void OnScoreChanged(int prevValue, int newValue)
    {
        ScoreUI.text = newValue.ToString(); 
    }
}

