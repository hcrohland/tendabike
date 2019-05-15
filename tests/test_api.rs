    
    
    use rocket::local::*;
    use rocket::http::{Header, Status};
    use tendabike::*;

    use tendabike::part::*;

    fn myreq<'c, 'u: 'c, U: Into<std::borrow::Cow<'u, str>>> (client: &'c Client, uri: U) -> LocalResponse<'c> {
        let response = client.get(uri).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        response
    }

    fn getparts<'a, 'b> (client: &'a Client, uri: &'b str) -> Vec<Part> {
        let mut response = myreq (client, uri);

        let myparts = serde_json::from_str(
                &response.body_string().expect("body is no string")
            ).expect("malformed body");
        myparts
    }

    /* use serde::de::Deserialize;
    fn getjson<'c, 'u: 'c, U: Into<std::borrow::Cow<'u, str>>, T: Deserialize<'u>> (req: &'c Client, uri: U) -> T {
        let mut response = req.get(uri).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let body = response.body_string().expect("body is no string");
        let myparts: T = serde_json::from_str(
                &body
            ).expect("malformed body");
        myparts
    } */

    #[test]
    fn part_types () {
            let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

            let mut response = myreq (&client, "/part/types");
            let types: Vec<PartTypes> = serde_json::from_str(&response.body_string().expect("")).expect("");
            assert_eq!(types.len(), 9);
            assert_eq!(types[0], PartTypes{id:1,name: String::from("Bike"), main:true, hooks: vec!(2,4,5,7,8)});
    }
    #[test]
    fn part () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/part/999").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let _myspares: Vec<Part> = getparts(&client, "/part/myspares");
        let myparts: Vec<Part> = getparts(&client, "/part/mygear");

        let mut response = myreq(&client, format!("/part/{}", myparts[0].id));
        let part: Part = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no part");
        assert_eq!(part.name.to_string(), "Bronson");

        let mut response = myreq (&client, format!("/part/{}?assembly", myparts[1].id));
        let ass: Assembly = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no assembly");
        assert_eq!(ass.part.name.to_string(), "Slide");
    }

    use tendabike::activity::*;

    #[test]
    fn activity_types () {
            let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

            let mut response = myreq(&client, "/activ/types");
            let types: Vec<ActivityType> = serde_json::from_str(&response.body_string().expect("")).expect("");
            assert!(types.len() > 0);
            assert_eq!(types[0], ActivityType {id:1,name: String::from("Bike Ride"), gear: 1});
    }
    #[test]
    fn activities () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/activ/999").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let mut response = myreq(&client, "/activ/1");
        let _part: Activity = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no activity");

    }
/* 
    #[test]
    fn usage () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let parts = getparts(&client, "/part/mygear");

        let mut response = myreq(&client, "/activ/1");
        let activity: Activity = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no activity");

        let part1 = &parts[0];
        let part2 = &parts[1];

        let (i1, i2) = match activity.id {
            part1.id => (part2.id, part1.id);
            part2.id => (part1.id, part2.id);
            _ => panic!("part not found {}", activity.id)
        }
    } */