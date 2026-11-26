import { mount } from "svelte";
import Example from "./Example.svelte";
// The app's real theme: the harness verifies against the same tokens the
// app renders with, so the two can never drift.
import "../src/app.css";

mount(Example, { target: document.getElementById("app")! });
