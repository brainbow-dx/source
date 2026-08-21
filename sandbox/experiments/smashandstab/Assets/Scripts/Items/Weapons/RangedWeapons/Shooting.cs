using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using Unity.Netcode;

public class Shooting : NetworkBehaviour
{
    [SerializeField]
    private PlayerController pc;

    private RangedWeaponDefinition gunData;

    private GameObject bulletObject;

    [SerializeField]
    private Inventory inventory;
    [SerializeField]
    private Camera mainCam;
    private Vector3 mousePos;
    private Transform bulletSpawnerTransform;
    [SerializeField]
    public Transform bulletPivotPointLeft;
    [SerializeField]
    public Transform bulletPivotPointRight;

    bool canFire = true; 

    [SerializeField]
    Animator _animator;

    [SerializeField]
    private float radius; //Radius around the player for bullet spawn

    private Vector3 v3Pos;

    void Start(){
        // bulletObject = gunData.bulletObject;

        // mainCam = GameObject.FindGameObjectWithTag("MainCamera").GetComponent<Camera>();

        // bulletSpawnerTransform = bulletObject.transform;

        // fix this later
        // _animator = this.transform.parent.GetComponentInChildren<Animator>(); 
        // Debug.Log(_animator);
    }

    void Update(){
        if(!IsOwner) return;
        mousePos = mainCam.ScreenToWorldPoint(Input.mousePosition);
        Aim();
        ChangePivotPoint();
        CheckAndFire();
    }

    IEnumerator Reload(){
        Debug.Log("Reloading...");
        yield return new WaitForSeconds(gunData._reloadRate);
        canFire = true;
    }

    // Fire bullet
    void FireBullet(){
        if (bulletObject != null) {
            GameObject bullet = Instantiate(bulletObject, bulletSpawnerTransform.position, Quaternion.identity);
            BulletScript bs = bullet.GetComponent<BulletScript>();
            bs.SetMousePos(mousePos); // Pass calculated information to BulletScript
            bs.SetGunData(gunData);
            inventory.SubtractBullet();
            Debug.Log("fired");
        } else {
            Debug.LogWarning("Bullet object is not set!");
        }
    }

    //check if can fire if so spawn bullet
    private void CheckAndFire(){
        if(gunData != null && Input.GetMouseButtonDown(0) && canFire){
            int bullets = inventory.GetBulletCount();
            if (bullets > 0){
                Debug.Log("Pressed left-click.");
                _animator.Play("shoot");
                FireBullet();
                canFire = false;
                StartCoroutine(Reload());
            }
            else {
                Debug.Log("Clip Empty!");
            }
        }
    }

    //rotate the bullettransform to aim at mouses location
    private void Aim( ){
        //calculate rotation angle relative to facing south
        Vector3 rotation = mousePos - transform.position;
        float rotZ = Mathf.Atan2(rotation.y, rotation.x) * Mathf.Rad2Deg;
        rotZ -= 90f;

        transform.rotation = Quaternion.Euler(0, 0, rotZ);
    }

    //Change pivot point according to whether running left or right
    private void ChangePivotPoint(){
        if (pc.lastDirection == true){
            RotateSpawnAroundAxis(bulletPivotPointLeft);
        }  else {
            RotateSpawnAroundAxis(bulletPivotPointRight);
        }
    }

    private void RotateSpawnAroundAxis(Transform bulletPivotPoint)
    {
        if (gunData != null && bulletObject != null){
            //calculate vector from pivot to mouse
            Vector3 direction = mousePos - bulletPivotPoint.position;
            
            //calculate angle between positive x-axis and the point (direction.x, direction.y)
            float angle = Mathf.Atan2(direction.y, direction.x) * Mathf.Rad2Deg;

            //normalize the angle
            if (angle < 0.0f) {
                angle += 360.0f;
            } 

            // transform.localEulerAngles = new Vector3(0, 0, angle);

            //compute cosine + sine giving x&y coords of unit circle then multiply by radius to scale it
            float xPos = Mathf.Cos(Mathf.Deg2Rad * angle) * radius;
            float yPos = Mathf.Sin(Mathf.Deg2Rad * angle) * radius;

            //set position
            bulletSpawnerTransform.position = new Vector3(bulletPivotPoint.position.x + xPos, bulletPivotPoint.position.y + yPos, 0);
        }
    }


    //getter & setter for bulletobject
    public void SetBulletObject(GameObject newBulletObject){
        bulletObject = newBulletObject;
    }

    public GameObject GetBulletObject(){
        return bulletObject;
    }

    //getter & setter for bullet spawner transform
    public void SetBulletTransform(Transform newBulletTransform){
        bulletSpawnerTransform = newBulletTransform;
    }

    public Transform GetBulletTransform(){
        return bulletSpawnerTransform;
    }

    public void UpdateWeaponData(RangedWeaponDefinition newWeaponData){
        gunData = newWeaponData;
        if (gunData != null){
            bulletObject = gunData.bulletObject;

            //initializes spawner
            bulletSpawnerTransform = bulletObject.transform;
        }
        
    }
}
