    
    
    use rocket::local::*;
    use rocket::http::{Header, Status, Method, ContentType};
    use chrono::Utc;
    use tendabike::*;

    use tendabike::part::*;

    use serde::{
            de::Deserialize,
            ser::Serialize
        };
    
    fn reqjson<'c, 'u, T, B, U> (client: &'c Client, method: Method, uri: U, body: B) -> T 
        where   for<'a> T: Deserialize<'a>, 
                B: Serialize,
                U: Into<std::borrow::Cow<'u, str>>,
    {
        let mut response = client.req(method, uri)
            .header(Header::new("x-user-id", "2"))
            .header(ContentType::JSON)
            .body(dbg!(serde_json::to_string(&body).unwrap()))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        serde_json::from_str::<T>(
                &response.body_string().expect("body is no string")
            ).expect("malformed body")
    } 

    fn getjson<'c, 'u, T, U> (client: &'c Client, uri: U) -> T 
        where for<'a> T: Deserialize<'a>, U: Into<std::borrow::Cow<'u, str>>,
    {
        reqjson(client, Method::Get, uri, "")
    }
    
    fn patchjson<'c, 'u, T, U> (client: &'c Client, uri: U) -> T 
        where for<'a> T: Deserialize<'a>, U: Into<std::borrow::Cow<'u, str>>,
    {
        reqjson(client, Method::Patch, uri, "")
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
        assert_eq!(ass.part.name.to_string(), "Slide");
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
    }

    #[test]
    fn usage () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let act: Activity = getjson(&client, "/activ/9");
        let ass1: Assembly = patchjson(&client, "/activ/9?gear=0");
        let response = client.patch("/activ/9?gear=-1").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        let ass4: Assembly = getjson(&client, format!("/part/{}?assembly", ass1.part.id));
        assert_eq!(ass1, ass4);
        let ass2: Assembly = patchjson(&client, "/activ/9");
        let ass3: Assembly = patchjson(&client, format!("/activ/9?gear={}",ass2.part.id));

        assert_eq!(ass2, ass3);
        assert_eq!(ass1.part.count + 1, ass2.part.count);
        assert_eq!(ass1.part.time + act.time.unwrap_or(0), ass2.part.time);

        let response = client.patch("/activ/9?gear=-1").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        let ass4: Assembly = getjson(&client, format!("/part/{}?assembly", ass2.part.id));
        assert_eq!(ass2, ass4);
    } 

    #[test]
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

        let act_new: Activity = reqjson(&client, Method::Put, "/activ/", &act);
        assert_ne!(act_new.id, 0);
        assert_eq!(act_new.start, act.start);

        let result: usize = reqjson(&client, Method::Delete, format!("/activ/{}",act_new.id), "");
        assert_eq!(result, 1);
        let result: usize = reqjson(&client, Method::Delete, "/activ/0", "");
        assert_eq!(result, 0);
    }