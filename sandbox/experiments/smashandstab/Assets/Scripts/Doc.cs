using UnityEngine;

//This isn't a component that's meant for the game; it's just useful for documentation.
public class Doc : MonoBehaviour
{
    [TextArea(3, 8)]
    [SerializeField] public string Notes;
}
