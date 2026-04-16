// Must be imported first for JIT compilation support
import '@angular/compiler';

// Zone.js should be imported before Angular modules
import 'zone.js';
import 'zone.js/testing';

// Now import Angular testing utilities
import { getTestBed } from '@angular/core/testing';
import {
  BrowserDynamicTestingModule,
  platformBrowserDynamicTesting,
} from '@angular/platform-browser-dynamic/testing';

// Initialize the Angular testing environment
getTestBed().initTestEnvironment(
  BrowserDynamicTestingModule,
  platformBrowserDynamicTesting()
);


