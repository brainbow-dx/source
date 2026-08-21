using UnityEngine;

public class DontDestroyOnLoadPersistent : MonoBehaviour
{
    // Start is called before the first frame update
    public void FixedUpdate()
    {
        DontDestroyOnLoad(gameObject);
    }
}
