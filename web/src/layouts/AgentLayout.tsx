import {
  IconCoinMoneyStroked,
  IconCustomerSupportStroked,
  IconExit,
  IconHomeStroked,
  IconList,
  IconStopwatchStroked,
  IconLineChartStroked,
  IconMenu,
  IconUserGroup
} from '@douyinfe/semi-icons';
import { Avatar, Button, Layout, Nav } from '@douyinfe/semi-ui';
import type { NavItems, OnSelectedData } from '@douyinfe/semi-ui/lib/es/navigation';
import { useEffect, useState, useSyncExternalStore, type CSSProperties, type ReactNode } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

import { authStore } from '../auth/authStore';
import { getAgentMe, type AgentMe } from '../api/agent';
import './AgentLayout.css';

const { Header, Sider, Content } = Layout;

type AgentNavItem = {
  icon: ReactNode;
  label: string;
  path: string;
};

const navItems: AgentNavItem[] = [
  { path: '/agent/dashboard', label: '总览', icon: <IconHomeStroked aria-hidden="true" /> },
  { path: '/agent/support', label: '在线客服', icon: <IconCustomerSupportStroked aria-hidden="true" /> },
  { path: '/agent/users', label: '团队用户', icon: <IconUserGroup aria-hidden="true" /> },
  { path: '/agent/orders/seconds', label: '秒合约订单', icon: <IconStopwatchStroked aria-hidden="true" /> },
  { path: '/agent/orders/margin', label: '合约订单', icon: <IconLineChartStroked aria-hidden="true" /> },
  { path: '/agent/orders/new-coin-subscriptions', label: '新币申购', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/orders/new-coin-purchases', label: '新币认购', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/orders/earn', label: '理财订单', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/orders/spot', label: '现货订单', icon: <IconList aria-hidden="true" /> },
  { path: '/agent/coin-applications', label: '新币发行申请', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/funding/deposits', label: '充值记录', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/funding/withdrawals', label: '提现记录', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/recharges', label: '我的充值申请', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/invite-codes', label: '邀请码', icon: <IconList aria-hidden="true" /> },
  { path: '/agent/commissions', label: '佣金记录', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/convert-stats', label: '闪兑统计', icon: <IconCoinMoneyStroked aria-hidden="true" /> },
  { path: '/agent/team-tree', label: '团队树', icon: <IconList aria-hidden="true" /> },
  { path: '/agent/sub-agents', label: '下级代理', icon: <IconUserGroup aria-hidden="true" /> }
];

const shellStyle: CSSProperties = {
  height: '100vh',
  minHeight: '100vh'
};

const navStyle: CSSProperties = {
  height: '100%',
  width: '100%'
};

const navBodyStyle: CSSProperties = {
  height: 'calc(100% - 64px)',
  overflowY: 'auto'
};

const headerStyle: CSSProperties = {
  alignItems: 'center',
  backgroundColor: 'var(--semi-color-bg-0)',
  borderBottom: '1px solid var(--semi-color-border)',
  display: 'flex',
  justifyContent: 'space-between',
  padding: '16px 24px'
};

const contentStyle: CSSProperties = {
  backgroundColor: 'var(--semi-color-bg-0)',
  minWidth: 0,
  overflow: 'auto'
};

const semiNavItems: NavItems = navItems.map((item) => ({
  icon: item.icon,
  itemKey: item.path,
  text: item.label
}));

function normalizePath(pathname: string): string {
  if (pathname === '/agent') return '/agent/dashboard';
  if (pathname.startsWith('/agent/users/')) return '/agent/users';
  return pathname;
}

function AgentHeaderIdentity() {
  const [profile, setProfile] = useState<AgentMe | null>(null);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    void getAgentMe(controller.signal).then((value) => {
      if (!controller.signal.aborted) setProfile(value);
    }).catch(() => {
      if (!controller.signal.aborted) setFailed(true);
    });
    return () => controller.abort();
  }, [attempt]);

  const name = profile?.username ?? '代理账号';
  const detail = profile
    ? `${['一级代理', '二级代理', '三级代理'][profile.level - 1]} · 代理编号 ${profile.agent_code}`
    : failed ? '账号资料暂不可用' : '账号资料加载中';

  return (
    <div className="agent-header-identity">
      <Avatar className="agent-header-avatar" size="small" aria-hidden="true">
        {profile ? Array.from(name)[0].toUpperCase() : '代'}
      </Avatar>
      <div className="agent-header-account" aria-live="polite" aria-busy={!profile && !failed}>
        <span className="agent-header-name" title={name}>{name}</span>
        <span className="agent-header-detail" title={detail} role={failed ? 'alert' : undefined}>{detail}</span>
      </div>
      {failed && (
        <Button size="small" theme="borderless" aria-label="重新加载代理账号资料" onClick={() => {
          setFailed(false);
          setAttempt((value) => value + 1);
        }}>重试</Button>
      )}
    </div>
  );
}

export function AgentLayout() {
  const [menuOpen, setMenuOpen] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('agent'));
  const activePath = normalizePath(location.pathname);
  const handleNavSelect = ({ itemKey }: OnSelectedData) => {
    const nextPath = String(itemKey);
    if (nextPath.startsWith('/agent')) {
      navigate(nextPath);
      setMenuOpen(false);
    }
  };

  return (
    <Layout className="semi-always-light agent-layout" style={shellStyle}>
      <Sider aria-label="代理侧边栏" className="agent-sidebar" data-expanded={menuOpen} style={{ width: 240 }}>
        <Nav
          aria-label="代理导航"
          bodyStyle={navBodyStyle}
          header={{
            logo: <Avatar size="small">AG</Avatar>,
            text: '代理门户'
          }}
          items={semiNavItems}
          mode="vertical"
          onSelect={handleNavSelect}
          selectedKeys={[activePath]}
          style={navStyle}
        />
      </Sider>
      <Layout style={{ minWidth: 0 }}>
        <Header className="agent-header" style={headerStyle}>
          <Button className="agent-menu-toggle" icon={<IconMenu />} theme="borderless"
            aria-label={menuOpen ? '收起代理菜单' : '展开代理菜单'} title={menuOpen ? '收起代理菜单' : '展开代理菜单'}
            aria-expanded={menuOpen} onClick={() => setMenuOpen((value) => !value)} />
          {session && <AgentHeaderIdentity key={`${session.subject}:${session.generation}`} />}
          <Button
            className="agent-header-logout"
            icon={<IconExit />}
            onClick={() => {
              authStore.clearSession('agent');
              navigate('/login', { replace: true });
            }}
            theme="borderless"
            type="tertiary"
          >
            退出登录
          </Button>
        </Header>
        <Content style={contentStyle}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
