    
    
    use rocket::local::*;
    use rocket::http::{Header, Status};
    use tendabike::*;

    use tendabike::part::*;

    use serde::de::Deserialize;
    fn getjson<'c, 'u, T, U> (client: &'c Client, uri: U) -> T 
        where for<'a> T: Deserialize<'a>, U: Into<std::borrow::Cow<'u, str>>,
    {
        let mut response = client.get(uri).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        serde_json::from_str::<T>(
                &response.body_string().expect("body is no string")
            ).expect("malformed body")
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

        let response = client.get("/part/999").header(Header::new("x-user-id", "2")).dispatch();
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

        let response = client.get("/activ/999").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let _part: Activity = getjson(&client, "/activ/1");
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