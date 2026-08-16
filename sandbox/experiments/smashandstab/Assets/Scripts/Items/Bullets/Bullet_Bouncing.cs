using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class Bullet_Bouncing : BulletScript
{
    [SerializeField]
    private float bulletAcceleration;
    private Vector3 lastVelocity;
    private int bounceCount;
    [SerializeField]
    public int maxBounces;
    private string newLayerName;
    

    // Start is called before the first frame update
    void Start()
    {
        base.Start();

        //bouncing bullet start
        bulletAcceleration = 3.0f;
        bounceCount = 0;
        maxBounces = 5;
        firstStruck = false;
        newLayerName = "Default";
    }

    // Update is called once per frame
    private void FixedUpdate() {
        lastVelocity = rb.linearVelocity;
    }

    protected override void OnCollisionEnter2D(Collision2D other){
        if(other.gameObject.CompareTag("Wall")){
            if (bounceCount == maxBounces){
                Destroy(this.gameObject);
                return;
            }
            bounceCount++;

            float speed = lastVelocity.magnitude + bulletAcceleration;
            
            //Calculate the direction of the reflection
            Vector2 direction = Vector3.Reflect(lastVelocity.normalized, other.contacts[0].normal);
            
            //Set the rotation to point in the direction of the reflection
            float angle = Mathf.Atan2(direction.y, direction.x) * Mathf.Rad2Deg;
            this.transform.rotation = Quaternion.Euler(0f, 0f, angle);

            //Freeze the rotation after every bounce
            rb.freezeRotation = true;

            rb.linearVelocity = direction * Mathf.Max(speed, 0f);

            //allow player to be killed by bounces
            firstStruck = true;
            this.gameObject.layer = LayerMask.NameToLayer(newLayerName); 
        }
        else if (other.gameObject.CompareTag("Player")){
            if (firstStruck == false){
                return;
            }
            else {
                Destroy(other.gameObject);
                Destroy(this.gameObject);
            }
        // } else if (other.gameObject.CompareTag("Range")){ //make it so that bullet does not destroy range
        //     return;
        } else {
            Destroy(other.gameObject);
            Destroy(this.gameObject);
        }
    }
}
