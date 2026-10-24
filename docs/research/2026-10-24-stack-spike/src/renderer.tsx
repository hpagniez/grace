import './index.css';
import { render } from '@solidjs/web';
import { createMemo, createSignal, Loading, onCleanup } from 'solid-js';
import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
  RouterProvider,
} from '@tanstack/solid-router';
import { Button } from '@kobalte/core/button';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const rootRoute = createRootRoute({
  component: () => (
    <main class="p-6 font-sans">
      <nav class="mb-4 flex gap-4 text-sm text-slate-500">
        <Link to="/">Home</Link>
        <Link to="/runs/$id" params={{ id: '42' }} data-test="run-link">
          Run 42
        </Link>
      </nav>
      <Outlet />
    </main>
  ),
});

function Home() {
  const [count, setCount] = createSignal(1);
  const [ticks, setTicks] = createSignal(0);
  onCleanup(window.api.onTick((n) => setTicks(n)));

  // Async memos: Solid 2 replacement for createResource.
  const versions = createMemo(() => window.api.getVersions());
  const doubled = createMemo(async () => {
    const c = count();
    await sleep(300);
    return c * 2;
  });

  return (
    <section class="space-y-2">
      <h1 class="text-3xl font-bold text-emerald-600">Home</h1>
      <p data-test="ticks">ticks: {ticks()}</p>
      <button class="rounded bg-emerald-600 px-3 py-1 text-white" onClick={() => setCount((c) => c + 1)}>
        count: {count()}
      </button>
      <Button data-test="kobalte" class="rounded border px-3 py-1" onClick={() => setCount((c) => c + 10)}>
        kobalte +10
      </Button>
      <Loading fallback={<p>loading...</p>}>
        <p data-test="doubled">doubled: {doubled()}</p>
        <p data-test="versions">
          electron {versions().electron} / chrome {versions().chrome} / node {versions().node}
        </p>
      </Loading>
    </section>
  );
}

function Run() {
  const params = runRoute.useParams();
  return <h1 class="text-3xl font-bold text-sky-600">Run {params().id}</h1>;
}

const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: Home });
const runRoute = createRoute({ getParentRoute: () => rootRoute, path: '/runs/$id', component: Run });

const router = createRouter({
  routeTree: rootRoute.addChildren([indexRoute, runRoute]),
  history: createHashHistory(),
});

declare module '@tanstack/solid-router' {
  interface Register {
    router: typeof router;
  }
}

render(() => <RouterProvider router={router} />, document.getElementById('root')!);
