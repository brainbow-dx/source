using UnityEngine;

namespace Aby
{
    public class TrapController : MonoBehaviour
    {
        [SerializeField] private int damage = 10;

        void OnCollisionEnter2D(Collision2D collision)
        {
            var character = collision.gameObject.GetComponent<CharacterDefinition>();
            if (character != null)
            {
                character.TakeDamage(damage);
            }
        }
    }
}
