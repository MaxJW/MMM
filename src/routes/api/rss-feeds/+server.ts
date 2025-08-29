// src/routes/api/rss/+server.ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import type { Article } from '$lib/types/rss';

const fakeArticles: Article[] = [
	{
		title: 'Llamas Take Over Willow Creek Park',
		source: 'Simlish Daily',
		date: new Date().toISOString()
	},
	{
		title: 'Mayor Announces Free Cupcakes for All Sims',
		source: 'Plumbob Times',
		date: new Date(Date.now() - 1000 * 60 * 60).toISOString()
	},
	{
		title: 'Breaking: Cowplant Spotted Near Goth Manor',
		source: 'Sul Sul News',
		date: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString()
	},
	{
		title: 'New Expansion: Unlimited Gnome Parties!',
		source: 'Sim City Gazette',
		date: new Date(Date.now() - 1000 * 60 * 60 * 5).toISOString()
	},
	{
		title: 'Sims Protest Lack of Pool Ladders',
		source: 'The Freezer Bunny Herald',
		date: new Date(Date.now() - 1000 * 60 * 60 * 12).toISOString()
	},
	{
		title: 'Bella Goth Still Missing: Neighbors Swear They Saw Her at the Pancake House',
		source: 'Mystery Gazette',
		date: new Date(Date.now() - 1000 * 60 * 60 * 20).toISOString()
	},
	{
		title: 'Rocket Crashes in Oasis Springs: Astronaut Claims "Just Testing Cheats"',
		source: 'SimNation Today',
		date: new Date(Date.now() - 1000 * 60 * 60 * 30).toISOString()
	},
	{
		title: 'Strangerville Residents Celebrate Annual "Talking Plant" Festival',
		source: 'Desert Bloom News',
		date: new Date(Date.now() - 1000 * 60 * 60 * 36).toISOString()
	},
	{
		title: 'Alien Sightings Increase: Sims Report Missing Grilled Cheese',
		source: 'Galactic Observer',
		date: new Date(Date.now() - 1000 * 60 * 60 * 48).toISOString()
	},
	{
		title: 'Breaking: Firefighters Finally Return to The Sims!',
		source: 'Simlish Daily',
		date: new Date(Date.now() - 1000 * 60 * 60 * 60).toISOString()
	},
	{
		title: 'Freezer Bunny Confirmed for Sims 5: Developers Say "About Time"',
		source: 'Plumbob Times',
		date: new Date(Date.now() - 1000 * 60 * 60 * 72).toISOString()
	},
	{
		title: 'Sul Sul! New Language School Opens in San Myshuno',
		source: 'Cultural Exchange Journal',
		date: new Date(Date.now() - 1000 * 60 * 60 * 84).toISOString()
	}
];

export const GET: RequestHandler = async () => {
	return json({ articles: fakeArticles });
};
