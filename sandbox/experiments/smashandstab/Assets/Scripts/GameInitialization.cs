using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class GameInitialization : MonoBehaviour
{

    [SerializeField]
    MapGenerator mapGenerator;



    // Start is called before the first frame update
    void Start()
    {
        // This is no longer needed.
        //mapGenerator.BeginGenerationClientRpc();
    }

}
