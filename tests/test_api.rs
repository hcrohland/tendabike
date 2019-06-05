    
    use part::Assembly;
    use rocket::local::*;
    use rocket::http::{Header, Status, Method, ContentType};
    use chrono::Utc;
    use tendabike::*;

    use tendabike::part::*;

    use serde::{
            de::Deserialize,
            ser::Serialize
        };

    use pretty_assertions::{assert_eq, assert_ne};
    
    fn reqjson<'c, 'u, T, B, U> (client: &'c Client, method: Method, uri: U, body: B, status: Status) -> T 
        where   for<'a> T: Deserialize<'a>, 
                B: Serialize,
                U: Into<std::borrow::Cow<'u, str>>,
    {
        let mut response = client.req(method, uri)
            .header(Header::new("x-user-id", "2"))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&body).unwrap())
            .dispatch();
        assert_eq!(response.status(), status);

        serde_json::from_str::<T>(
                &response.body_string().expect("body is no string")
            ).expect("malformed body")
    } 

    fn getjson<'c, 'u, T, U> (client: &'c Client, uri: U) -> T 
        where for<'a> T: Deserialize<'a>, U: Into<std::borrow::Cow<'u, str>>,
    {
        reqjson(client, Method::Get, uri, "", Status::Ok)
    }
    
    fn patchjson<'c, 'u, T, U> (client: &'c Client, uri: U) -> T 
        where for<'a> T: Deserialize<'a>, U: Into<std::borrow::Cow<'u, str>>,
    {
        reqjson(client, Method::Patch, uri, "", Status::Ok)
    }

    #[test]
    fn part_types () {
            let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

            let types: Vec<PartTypes> = getjson (&client, "/part/types");
            assert_eq!(types[0], PartTypes{id:1,name: String::from("Bike"), main:true, hooks: vec!(2,4,5,7,8)});
    }
    #[test]
    fn part () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/part/0").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let _myspares: Vec<Part> = getjson(&client, "/part/myspares");
        let myparts: Vec<Part> = getjson(&client, "/part/mygear");

        let part: Part = getjson(&client, format!("/part/{}", myparts[0].id));
        assert_eq!(part.name.to_string(), "Bronson");

        let ass: Assembly = getjson (&client, format!("/part/{}?assembly", myparts[1].id));
        assert_eq!(ass.get(&myparts[1].id).unwrap().name.to_string(), "Slide");
    }

    use tendabike::activity::*;

    #[test]
    fn activity_types () {
            let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

            let types: Vec<ActivityType> = getjson(&client, "/activ/types");
            assert_eq!(types[0], ActivityType {id:1,name: String::from("Bike Ride"), gear: 1});
    }
    #[test]
    fn activities () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/activ/0").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let _part: Activity = getjson(&client, "/activ/1");
        reassign_activities();
        post_and_delete_activity();
    }

  
    fn reassign_activities () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        // get the activity
        let act: Activity = getjson(&client, "/activ/9");
        // Deregister Activitiy from any gear
        patchjson::<Assembly,_>(&client, "/activ/9?gear=");
        let ass: Assembly = getjson(&client, "/part/1?assembly");
        let part0 = ass.get(&1).unwrap();
        // Now register it to gear 1
        let ass: Assembly = patchjson(&client, "/activ/9?gear=1");
        // gear 1 has to be in the result. Get it!
        let part1 = ass.get(&1).unwrap();
        // Make sure you get a NotFound wen trying to register to a non-existing gear
        let response = client.patch("/activ/9?gear=-1").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        // Make sure that the patch result matches the stored assembly
        let ass: Assembly = getjson(&client, format!("/part/{}?assembly", part1.id));
        let part2 = ass.get(&1).unwrap();
        assert_eq!(part1, part2);
        assert_eq!(part0.count + 1, part1.count);
        assert_eq!(part0.time + act.time.unwrap_or(0), part1.time);
    } 
    
    fn post_and_delete_activity () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let act = NewActivity {
            user_id: 2,
            name:   String::from("test activity"),
            what:   1,
            gear:   Some(1),
            start:  Utc::now(),
            duration: 70,
            time:   Some(60),
            climb:  Some(1000),
            distance: Some(20000),
            descend: None,
            power: None,
        };

        let mut act_new: Activity = reqjson(&client, Method::Post, "/activ/", &act, Status::Created);
        assert_ne!(act_new.id, 0);
        assert_eq!(act_new.start, act.start);
     
        act_new.descend = Some(555);

        let act_3: Activity = reqjson(&client, Method::Put, format!("/activ/{}", act_new.id), &act_new, Status::Ok); //Should use response header
        assert_eq!(act_new.id, act_3.id);
        assert_eq!(act_new.start, act_3.start);
        assert_eq!(act_3.descend, Some(555));

        let act_del: Activity = reqjson(&client, Method::Delete, format!("/activ/{}",act_new.id), "", Status::Ok);
        assert_eq!(act_del.name, act.name);
    }