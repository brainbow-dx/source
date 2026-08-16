using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.SceneManagement;

public class SceneChanger : MonoBehaviour
{
    public void gotoGame()
    {
        SceneManager.LoadScene(3);
    }

    public void gotoBetween()
    {
        SceneManager.LoadScene(2);
    }
}
