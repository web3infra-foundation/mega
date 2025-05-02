import React, { ComponentType } from 'react';
import Layout1 from './layout1/Layout1';
import Layout2 from './layout2/Layout2';
import Layout3 from './layout3/Layout3';

/**
 * The type definition for the theme layouts.
 */
export type themeLayoutsType = Record<string, ComponentType<{ children?: React.ReactNode }>>;

/**
 * The theme layouts.
 */
const themeLayouts: themeLayoutsType = {
	layout1: Layout1,
	layout2: Layout2,
	layout3: Layout3
};

export default themeLayouts;
