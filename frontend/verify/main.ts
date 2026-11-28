import { mount } from "svelte";
import Example from "./Example.svelte";
// The app's real theme (via theme.css, which also points Tailwind's
// candidate scan at src/): the harness verifies against the same tokens the
// app renders with, so the two can never drift.
import "./theme.css";

mount(Example, { target: document.getElementById("app")! });
