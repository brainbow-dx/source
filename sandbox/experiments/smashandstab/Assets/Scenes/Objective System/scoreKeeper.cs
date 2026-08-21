using System.Collections;
using System.Collections.Generic;
using UnityEngine;

[CreateAssetMenu]
public class scoreKeeper : ScriptableObject
{
    // Start is called before the first frame update
    public List<int> scores;
    public int players;

    public void AddPlayer()
    {
        scores.Add(0);
        players += 1;
    }

    public void ClearSave()
    {
        scores.Clear();
        players = 0;
    }


    public List<int> getScores()
    {
        return scores;
    }

    public int getNumPlayers()
    {
        return players;
    }
}
