using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class lobby : MonoBehaviour
{
    [SerializeField]
    //Where we spawn UI elements
    private GameObject grid;

    [SerializeField]
    // A scriptable object to store data between scenes
    private scoreKeeper scoreS0;

    [SerializeField]
    //UI elements to be spawned
    private GameObject playerDisplay;

    private int numPlayers;
    private int maxPlayers = 8;

    private void Start()
    {
        numPlayers += 1;
        GameObject player = Instantiate(playerDisplay, this.transform.position, Quaternion.identity, grid.transform);
        player.GetComponent<lobbyPlayerUI>().intialize("Player 1 (Host)");
        scoreS0.ClearSave();
        scoreS0.AddPlayer();
    }

    public void AddPlayer()
    {
        if (numPlayers >= maxPlayers)
        {
            Debug.Log("Already at max Players");
        }
        else
        {
            numPlayers += 1;
            GameObject player = Instantiate(playerDisplay, this.transform.position, Quaternion.identity, grid.transform);
            player.GetComponent<lobbyPlayerUI>().intialize("Player " + numPlayers.ToString());
            scoreS0.AddPlayer();
        }
    }
}
