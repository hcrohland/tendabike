<script lang="ts">
  import {
    Card,
    CardBody,
    CardFooter,
    CardHeader,
  } from "@sveltestrap/sveltestrap";
</script>

<div class="row p-sm-2">
  <Card>
    <CardHeader>
      <h2>
        Tend your bike for a happy ride
        <div class="float-end">
          <a href="http://www.strava.com" target="_blank">
            <img
              src="api_logo_pwrdBy_strava.png"
              alt="powered by Strava"
              height="50"
            />
          </a>
        </div>
      </h2>
      Know how long your parts last and when to maintain them
    </CardHeader>
    <CardBody>
      <h4>General use</h4>
      <p>
        TendaBike fetches all your activities and bikes from Strava.
        <br /> You can create parts and attach them to your bikes. You can move
        parts between bikes.
        <br /> TendaBike will calculate the use for any part you create.
        <br /> You can also add service plans and record services and you will get
        reminded if a service plan is due
      </p>
      <p>
        When initially connecting TendaBike to Strava it will pull all bikes and
        activities into the database. Don't be afraid. You can correct anything
        later. Changes on Strava will be replicated into TendaBike
        <br /> After connecting you can start adding service plans, part and services.
        This is easiest done from the bike's detail page which you can find by clicking
        on the arrow right of the bike's name
      </p>
      <h4>Elevation data</h4>
      <p>
        Strava only tracks elevation gain. This is unfortunate if you use lifts
        or shuttles.
        <br />Brakes are wearing off way faster downhill than uphill while e.g.
        chains wear on the uphill.
        <br /> If you are using a recent Garmin device and Garmin connect, we have
        you covered!
      </p>
      <p>This is what you can do:</p>
      <ol>
        <li>
          Stop recording in lifts and shuttles. By that Strava should not track
          the elevation gain for the shuttle
        </li>
        <li>
          Download the list of activities from the Garmin Connect website with
          German language settings.
          <br /> The resulting file will include the elevation loss
        </li>
        <li>
          Upload this file to TendaBike via 'Sync' &rightarrow; 'With CSV file'
        </li>
      </ol>
      <p></p>
    </CardBody>
    <CardFooter>
      © Christoph Rohland 2019-2024 - Source Code at
      <a href="https://github.com/hcrohland/tendabike" title="Repository"
        >GitHub</a
      >
    </CardFooter>
  </Card>
</div>
