import { mount } from "svelte";
import ChipFill from "../src/routes/prototype/chip-fill/ChipFill.svelte";
// The app's real theme (via theme.css, which also points Tailwind's
// candidate scan at src/): the harness renders the same tokens the app does.
import "./theme.css";

mount(ChipFill, { target: document.getElementById("app")! });
