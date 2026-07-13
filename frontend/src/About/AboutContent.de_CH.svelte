<script lang="ts">
  import { Heading, Hr, Li, List, P } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
</script>

<div class="p-4 sm:p-6 md:p-8 m-8 max-w-200">
  <a href="http://www.strava.com" class="float-end items-start" target="_blank">
    <img
      src="api_logo_pwrdBy_strava.png"
      alt="understützt vo Strava"
      height="20"
    />
  </a>

  <Heading tag="h3">Pfläg dis Velo für e sorgefreii Fahrt</Heading>
  <P
    >Behalt im Blick, wie lang dini Teili haltet und wenn si gwartet werde müend</P
  >
  <Heading tag="h5">Allgemeini Nutzig</Heading>
  <P>
    TendaBike lädt alli dini Aktivitäte und Velos vo Strava.
    <br /> Du chasch Teili aalege und a dine Velos aabaue. Du chasch Teili
    zwüsche Velos verschiebe.
    <br /> TendaBike bereched d'Nutzig für jedes Teil, wo du aalegsch.
    <br /> Du chasch usserdem Wartigspläni aalege und Services iitrage. Du wirsch
    erinneret, sobald e Wartigsplan fällig isch.
  </P>
  <P>
    Wenn du TendaBike zum erste Mal mit Strava verbindsch, werded alli Velos und
    Aktivitäte in d'Datebank glade. Kei Sorge, du chasch spöter alles
    korrigiere. Änderige uf Strava werded au i TendaBike übernoh.
    <br /> Nachdem Verbinde chasch Wartigspläni, Teili und Services derzuefüege. Am
    eifachste gaht das über d'Detailsiite vomene Velo, wo du über dr Pfeil rächts
    näbe em Veloname erreichsch.
  </P>

  <Heading tag="h5">Höchedate</Heading>
  <P>
    Strava erfasst nur dr Höchegwinn. Das isch ungünstig, wenn du Lift oder
    Shuttles benutzisch.
    <br />Brämse verschleisse bergab dütlich schneller als berguf, während z. B.
    Chette eher bim Bergufffahre verschleisse.
    <br /> Wenn du es aktuells Garmin-Gerät und Garmin Connect benutzisch, hend mir
    e Lösig für dich!
  </P>
  <P>Das chasch du mache:</P>
  <P>
    <List tag="ol">
      <Li>
        Stopp d'Ufzeichnig i Lift und Shuttles. Dadurch sött Strava dr
        Höchegwinn vom Shuttle nüme erfasse.
      </Li>
      <Li>
        Lad d'Aktivitätsliste vo dr Garmin-Connect-Website mit dütsche
        Spracheinstellige ache.
        <br /> Die Datei wo drus entstaht, het dr Höcheverlust drin.
      </Li>
      <Li>
        Lad die Datei über '{m.header_sync()}' &rightarrow; '{m.header_csv()}' i
        TendaBike ui.
      </Li>
    </List>
  </P>
  <Hr />
  <P>
    <div class="flex justify-end">
      © Christoph Rohland 2019-2026 - Sourcecode uf&MediumSpace;
      <a href="https://github.com/hcrohland/tendabike" title="Repository">
        GitHub
      </a>
    </div>
  </P>
</div>
