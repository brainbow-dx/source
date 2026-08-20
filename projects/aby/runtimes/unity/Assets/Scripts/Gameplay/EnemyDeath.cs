using Platformer.Core;
using Platformer.Mechanics;

namespace Platformer.Gameplay
{
    /// <summary>
    /// Fired when the health component on an enemy has a hitpoint value of  0.
    /// </summary>
    /// <typeparam name="EnemyDeath"></typeparam>
    public class EnemyDeath : Simulation.Event<EnemyDeath>
    {
        public EnemyController enemy;
        public PlayerController player;

        public override void Execute()
        {
            // Call the Die method on the enemy which triggers the death animation and fades out the enemy.
            enemy.Die();
        }
    }
}