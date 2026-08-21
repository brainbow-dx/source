using System.Collections;
using Platformer.Gameplay;
using UnityEngine;
using static Platformer.Core.Simulation;

namespace Platformer.Mechanics
{
    /// <summary>
    /// A simple controller for enemies. Provides movement control over a patrol path.
    /// </summary>
    [RequireComponent(typeof(AnimationController), typeof(Collider2D))]
    public class EnemyController : MonoBehaviour
    {
        public PatrolPath path;
        public AudioClip ouch;

        internal PatrolPath.Mover mover;
        internal AnimationController control;
        internal Collider2D _collider;
        internal AudioSource _audio;
        internal Animator animator;
        SpriteRenderer spriteRenderer;

        protected bool isDead = false;
        public bool IsDead => isDead;

        public Bounds Bounds => _collider.bounds;

        /// <summary>
        /// The way the enemy's sprite is currently facing, used by <see
        /// cref="PlayerEnemyCollision"/> to resolve which side of a collision is attacking.
        /// </summary>
        public Vector3 Direction => spriteRenderer.flipX ? Vector3.right : Vector3.left;

        /// <summary>
        /// Set by <see cref="PlayerEnemyCollision"/> on collision with the player, not by this
        /// class itself.
        /// </summary>
        public bool IsAttacking = false;

        void Awake()
        {
            control = GetComponent<AnimationController>();
            _collider = GetComponent<Collider2D>();
            _audio = GetComponent<AudioSource>();
            spriteRenderer = GetComponent<SpriteRenderer>();
            animator = GetComponent<Animator>();
        }

        public void Die()
        {
            if (isDead) return;

            // Note: We'll want to trigger hurt seperately from death
            //   when we implement heath for all the enemies.
            // TODO: Move this to a seperate interaction.
            animator.SetTrigger("hurt");

            isDead = true;
            TotalDeaths++;
            if (_audio && ouch)
            {
                _audio.PlayOneShot(ouch);
            }
            gameObject.layer = LayerMask.NameToLayer("Ghosts");
            StartCoroutine(DelayedDeath());
        }

        private IEnumerator DelayedDeath()
        {
            control.enabled = false;

            animator.SetTrigger("death");

            // Wait for the animation to finish.
            yield return new WaitForSeconds(1.5f);

            StartCoroutine(Despawn());
        }

        private int TotalDeaths = 0;
        public float despawnDelay = 60f;

        IEnumerator Despawn()
        {
            // Wait to remove the corpse from the scene.
            yield return new WaitForSeconds(despawnDelay);

            var sprite = GetComponent<SpriteRenderer>();
            var originalColor = sprite.color;
            var fadeDuration = 0.25f;
            for (float t = 0; t < 1; t += Time.deltaTime / fadeDuration)
            {
                sprite.color = new Color(originalColor.r, originalColor.g, originalColor.b, Mathf.Lerp(1, 0, t));
                yield return null;
            }

            gameObject.SetActive(false);
        }

        void OnCollisionEnter2D(Collision2D collision)
        {
            var player = collision.gameObject.GetComponent<PlayerController>();
            if (player != null)
            {
                var ev = Schedule<PlayerEnemyCollision>();
                ev.player = player;
                ev.enemy = this;
            }
        }

        void Patrol()
        {
            if (mover == null) mover = path.CreateMover(control.maxSpeed * 0.5f);
            control.move.x = Mathf.Clamp(mover.Position.x - transform.position.x, -1, 1);
        }

        protected virtual void Update()
        {
            if (!isDead && path != null)
            {
                Patrol();
            }
        }
    }
}
