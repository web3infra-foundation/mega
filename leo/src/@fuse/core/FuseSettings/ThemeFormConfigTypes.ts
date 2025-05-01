type RadioOptionType = {
	name: string;
	value: string;
};

type FormFieldBaseType = {
	title: string;
};

type RadioFieldType = FormFieldBaseType & {
	type: 'radio';
	options: RadioOptionType[];
};

type NumberFieldType = FormFieldBaseType & {
	type: 'number';
	min?: number;
	max?: number;
};

type SwitchFieldType = FormFieldBaseType & {
	type: 'switch';
};

type GroupFieldChildrenType = Record<string, RadioFieldType | SwitchFieldType | NumberFieldType | GroupFieldType>;

/**
 * The GroupFieldType type defines the shape of a group form field.
 * It extends the FormFieldBaseType type and adds a children property which is a GroupFieldChildrenType object.
 */
type GroupFieldType = FormFieldBaseType & {
	type: 'group';
	children: GroupFieldChildrenType;
};

export type AnyFormFieldType = RadioFieldType | SwitchFieldType | NumberFieldType | GroupFieldType;

/**
 * The ThemeFormConfigTypes type is an object where the keys are strings and the values are AnyFormFieldType objects.
 * It is used to generate the form fields based on the configuration in the themeLayoutConfigs object.
 */
type ThemeFormConfigTypes = Record<string, AnyFormFieldType>;

export default ThemeFormConfigTypes;
