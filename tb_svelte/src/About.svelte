<script>
  import {Jumbotron} from 'sveltestrap';
  import {
    Card,
    CardBody,
    CardFooter,
    CardHeader,
    CardSubtitle,
    CardText,
    CardTitle
  } from 'sveltestrap';

</script>

<div class="row p-sm-2">

  <Card>
    <CardHeader>
      <h2>
        Tend your bike for a happy ride
        <div class="float-right">
          <a href="http://www.strava.com" target="_blank">
            <img src="api_logo_pwrdBy_strava.png" alt="powered by Strava" height="50"> 
          </a>
        </div>
      </h2>
      Know how long your parts last and when to maintain them
    </CardHeader>
    <CardBody>
      
      <h4>
        General use
      </h4>
      <p>
        TendaBike fetches all your activities and bikes from Strava.
        <br> You can create parts and attach them to your bikes. You can move parts between bikes.
        <br> TendaBike will calculate the use for any part you create.
      </p>
      <p>
        On login, TendaBike will synchronize with Strava. With that it populates the bike list. 
        <br> To create parts switch to "Spare Parts". Here you can create and attach parts.
        <br> To see the bike details click on the arrow next to the bike's name
      </p>
      <h4>
        Elevation data
      </h4>
      <p>
        Strava only tracks elevation gain. This is unfortunate if you use lifts or shuttles.
        <br>Brakes are wearing off way faster downhill than uphill while e.g. chains wear on the uphill.
        <br> If you are using a recent Garmin device and Garmin connect, we have you covered!
      </p>
      <p>
        This is what you can do:
      </p>
      <ol>
        <li>Stop recording in lifts and shuttles. By that Strava should not track the elevation gain for the shuttle</li>
        <li>Download the list of activities from the Garmin Connect website with German language settings. 
          <br> The resulting file will include the elevation loss
        </li>
        <li>Upload this file to TendaBike via 'Sync' &rightarrow; 'Garmin' 
          <br>You can set the timezone of the uploaded activities. It applies to all activities in the file
        </li>
      </ol>
      <p></p>  
    </CardBody>
    <CardFooter>
      © Christoph Rohland 2019-2020
      - Icons made by <a href="https://www.flaticon.com/authors/freepik" title="Freepik">Freepik</a> 
      from <a href="https://www.flaticon.com/" title="Flaticon">www.flaticon.com</a>
    </CardFooter>
  </Card>
</div>