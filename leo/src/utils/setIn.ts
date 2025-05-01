import _ from 'lodash';

type State = Record<string, unknown> | Record<string, unknown>[];

const setIn = (state: State, name: string, value: unknown): State => {
	return _.setWith(_.clone(state), name, value);
};

export default setIn;
