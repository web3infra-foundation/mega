import React from 'react';
import { MenuItem, Select, ListItemIcon, ListItemText, Typography, useTheme } from '@mui/material';

type ProjectOption = {
	value: string;
	logo: string;
	darkLogo: string;
	name: string;
	url: string;
};

const projectOptions: ProjectOption[] = [
	{
		value: 'Vitejs',
		logo: '/assets/images/logo/vite.svg',
		darkLogo: '/assets/images/logo/vite.svg',
		name: 'Vitejs',
		url: 'https://fuse-react-vitejs-skeleton.fusetheme.com'
	},
	{
		value: 'Nextjs',
		logo: '/assets/images/logo/nextjs.svg',
		darkLogo: '/assets/images/logo/nextjs-dark.svg',
		name: 'Nextjs',
		url: 'https://fuse-react-nextjs-skeleton.fusetheme.com'
	}
];

function MainProjectSelection() {
	const [selectedProjectValue, setSelectedProject] = React.useState<string>(projectOptions[1].value);
	const selectedProject = projectOptions.find((project) => project.value === selectedProjectValue);
	const theme = useTheme();
	const handleMenuItemClick = (projectValue: string) => {
		setSelectedProject(projectValue);

		const selectedProjectUrl = projectOptions.find((project) => project.value === projectValue)?.url;

		if (typeof window !== 'undefined' && selectedProjectUrl) {
			const currentUrl = new URL(window.location.href);
			const newUrl = selectedProjectUrl + currentUrl.pathname;

			window.location.href = newUrl;
		}
	};

	return (
		<Select
			value={selectedProjectValue}
			onChange={(event) => handleMenuItemClick(event.target.value)}
			displayEmpty
			renderValue={(_selectedValue) => (
				<div style={{ display: 'flex', alignItems: 'center' }}>
					<img
						src={theme.palette.mode === 'dark' ? selectedProject.darkLogo : selectedProject.logo}
						alt={`${selectedProject.name} Logo`}
						width={20}
						height={20}
						style={{ marginRight: 8 }}
					/>
					<Typography className="text-md font-semibold">{selectedProject.name}</Typography>
				</div>
			)}
			sx={{
				backgroundColor: 'transparent',
				'& .MuiInputBase-input': {
					padding: '0 22px 0 8px!important'
				},
				'& .MuiSelect-icon': {
					width: 20,
					right: 1
				}
			}}
			size="small"
		>
			{projectOptions.map((project) => (
				<MenuItem
					key={project.value}
					value={project.value}
				>
					<ListItemIcon>
						<img
							src={theme.palette.mode === 'dark' ? project.darkLogo : project.logo}
							alt={`${project.name} Logo`}
							width={20}
							height={20}
						/>
					</ListItemIcon>
					<ListItemText
						primary={project.name}
						classes={{ primary: 'text-md font-semibold' }}
					/>
				</MenuItem>
			))}
		</Select>
	);
}

export default MainProjectSelection;
