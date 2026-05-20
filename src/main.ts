import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
import './lib/print/print.css';

const app = mount(App, { target: document.getElementById('app')! });
export default app;
