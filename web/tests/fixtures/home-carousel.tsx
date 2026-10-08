// Local-only actual editor fixture; every HTTP request is isolated.
import '@douyinfe/semi-ui/react19-adapter';
import '@douyinfe/semi-ui/lib/es/_base/base.css';
import '../../src/styles.css';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { AdminAccessProvider } from '../../src/admin/access';
import { HomeCarouselPage } from '../../src/admin/actions/HomeCarouselPage';

const imageUrl = new URL('/src/assets/brand/hippo-logo-compact.png', location.origin).href;
let config = {
  revision: 1,
  slides: [{
    id: 'invite', position: 0, enabled: true, locale: 'zh-CN', title: '邀请好友',
    description: '分享专属邀请码，邀请好友加入。', image_url: imageUrl,
    target: { type: 'referrals', id: null }
  }]
};
window.fetch = async (input, init) => {
  if (String(input).includes('/admin/api/v1/uploads/images')) {
    return Response.json({
      download_url: imageUrl, mime_type: 'image/png', object_key: 'local-fixture',
      provider: 'local', size_bytes: 1024
    });
  }
  if (!String(input).includes('/admin/api/v1/home/carousel')) return Response.json({}, { status: 404 });
  if (init?.method === 'PATCH') {
    const request = JSON.parse(String(init.body));
    if (request.expected_revision !== config.revision) return Response.json({ code: 'CONFLICT', message: 'revision changed' }, { status: 409 });
    config = { revision: config.revision + 1, slides: request.slides };
  }
  return Response.json(config);
};
const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const router = createMemoryRouter([{ path: '/', element: <HomeCarouselPage /> }]);
createRoot(document.getElementById('root')!).render(
  <AdminAccessProvider access={{
    admin_id: 1, username: 'local-editor', role_id: 1, role_name: 'Local',
    is_super_admin: false, permissions: ['content.news.read', 'content.news.write', 'system.uploads.write']
  }}>
    <QueryClientProvider client={queryClient}><RouterProvider router={router} /></QueryClientProvider>
  </AdminAccessProvider>
);
