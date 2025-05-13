<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
</script>

<div class="container mx-auto py-8">
	<Card class="mx-auto max-w-xl">
		<CardHeader>
			<CardTitle class="text-center text-2xl">Mus Music Player</CardTitle>
		</CardHeader>
		<CardContent>
			<div class="flex flex-col items-center space-y-4">
				<p class="text-center">Welcome to your personal music player</p>
				<Button>Start Listening</Button>
			</div>
		</CardContent>
	</Card>
</div>
