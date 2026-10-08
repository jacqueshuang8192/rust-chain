import { IconArrowDown, IconArrowUp, IconDelete, IconPlus } from '@douyinfe/semi-icons';
import { Button, Switch, Tooltip, Typography } from '@douyinfe/semi-ui';
import { useMemo } from 'react';
import { useCanAdminRequest } from '../access';
import { AdminImageUpload } from '../../shared/AdminImageUpload';
import { AdminSelect, AdminTextArea, AdminTextInput } from '../../shared/SemiFormControls';
import { AdminSettingsPage, buildSettingsDifferences, SettingsSaveConfirmation, useAdminSettingsEditor } from '../settings';
import {
  carouselFields, emptySlide, homeCarouselApiPath, loadCarouselConfig, maxSlides,
  saveCarouselConfig, targetOptions, validateCarouselForm,
  type CarouselSlideDraft, type HomeCarouselForm, type HomeCarouselTargetType
} from './homeCarouselModel';
import './HomeCarouselPage.css';

export function HomeCarouselPage() {
  const canWrite = useCanAdminRequest(homeCarouselApiPath, 'PATCH');
  const editor = useAdminSettingsEditor<HomeCarouselForm, HomeCarouselForm>({
    settingKey: homeCarouselApiPath,
    initialForm: { revision: 0, slides: [] },
    load: loadCarouselConfig,
    selectForm: (data) => data,
    save: saveCarouselConfig,
    successMessage: '首页轮播配置已保存。'
  });
  const form = editor.draft;
  const disabled = !canWrite || editor.isSaving || editor.isFetching;
  const differences = useMemo(() => buildSettingsDifferences(
    editor.baseline ?? form, form, carouselFields(editor.baseline ?? form, form)
  ), [editor.baseline, form]);
  const validationIssues = validateCarouselForm(form);

  function changeSlide(id: string, update: Partial<CarouselSlideDraft>) {
    if (disabled) return;
    editor.setDraft((current) => ({
      ...current, slides: current.slides.map((slide) => slide.id === id ? { ...slide, ...update } : slide)
    }));
  }

  function moveSlide(index: number, direction: number) {
    if (disabled || index + direction < 0 || index + direction >= form.slides.length) return;
    editor.setDraft((current) => {
      const slides = [...current.slides];
      [slides[index], slides[index + direction]] = [slides[index + direction], slides[index]];
      return { ...current, slides };
    });
  }

  return (
    <AdminSettingsPage title="首页轮播" feedback={editor.feedback} isDirty={editor.isDirty}
      isInitialLoading={editor.isInitialLoading} isReady={editor.isReady} isRefreshing={editor.isFetching}
      loadError={editor.loadError} onReload={editor.isSaving ? async () => {} : editor.reloadLatest}>
      <div className="admin-carousel-page">
        <div className="admin-carousel-toolbar">
          <Typography.Text>版本 {form.revision} · {form.slides.length}/{maxSlides} 项</Typography.Text>
          {canWrite && <Button disabled={disabled || form.slides.length >= maxSlides} icon={<IconPlus aria-hidden />}
            onClick={() => editor.setDraft((current) => ({ ...current, slides: [...current.slides, emptySlide()] }))}
            theme="solid">新增轮播项</Button>}
        </div>
        {!form.slides.length && <div className="admin-carousel-empty">暂无轮播内容</div>}
        {form.slides.map((slide, index) => (
          <fieldset className="admin-carousel-item" disabled={disabled} key={slide.id}>
            <legend>第 {index + 1} 项</legend>
            <div className="admin-carousel-item__header">
              <label className="admin-carousel-toggle">启用
                <Switch aria-label={`第 ${index + 1} 项启用`} checked={slide.enabled} disabled={disabled}
                  onChange={(enabled) => changeSlide(slide.id, { enabled })} />
              </label>
              {canWrite && <div className="admin-carousel-commands">
                <Tooltip content="上移"><Button aria-label={`上移第 ${index + 1} 项`} disabled={disabled || index === 0}
                  icon={<IconArrowUp aria-hidden />} onClick={() => moveSlide(index, -1)} theme="borderless" /></Tooltip>
                <Tooltip content="下移"><Button aria-label={`下移第 ${index + 1} 项`} disabled={disabled || index === form.slides.length - 1}
                  icon={<IconArrowDown aria-hidden />} onClick={() => moveSlide(index, 1)} theme="borderless" /></Tooltip>
                <Tooltip content="删除"><Button aria-label={`删除第 ${index + 1} 项`} disabled={disabled}
                  icon={<IconDelete aria-hidden />} theme="borderless" type="danger"
                  onClick={() => editor.setDraft((current) => ({ ...current, slides: current.slides.filter((item) => item.id !== slide.id) }))} /></Tooltip>
              </div>}
            </div>
            <div className="admin-action-form admin-carousel-form">
              <label>标题<AdminTextInput ariaLabel={`第 ${index + 1} 项标题`} value={slide.title} disabled={disabled}
                onChange={(title) => changeSlide(slide.id, { title })} /></label>
              <label>语言<AdminSelect ariaLabel={`第 ${index + 1} 项语言`} disabled={disabled} value={slide.locale}
                optionList={[{ value: '', label: '全部语言' }, { value: 'zh-CN', label: '简体中文' }, { value: 'en-US', label: '英语' },
                  ...(!['', 'zh-CN', 'en-US'].includes(slide.locale) ? [{ value: slide.locale, label: slide.locale }] : [])]}
                onChange={(locale) => changeSlide(slide.id, { locale })} /></label>
              <label className="admin-carousel-form__full">描述<AdminTextArea ariaLabel={`第 ${index + 1} 项描述`}
                value={slide.description} autosize onChange={(description) => changeSlide(slide.id, { description })} /></label>
              <label>目标类型<AdminSelect ariaLabel={`第 ${index + 1} 项目标类型`} disabled={disabled}
                value={slide.targetType} optionList={targetOptions}
                onChange={(targetType) => changeSlide(slide.id, { targetType: targetType as HomeCarouselTargetType, newsId: '' })} /></label>
              {slide.targetType === 'news_detail' && <label>新闻 ID<AdminTextInput ariaLabel={`第 ${index + 1} 项新闻 ID`}
                disabled={disabled} value={slide.newsId} onChange={(newsId) => changeSlide(slide.id, { newsId })} /></label>}
              <label className="admin-carousel-form__full">图片地址<AdminTextInput ariaLabel={`第 ${index + 1} 项图片地址`}
                disabled={disabled} value={slide.imageUrl} onChange={(imageUrl) => changeSlide(slide.id, { imageUrl })} /></label>
              {canWrite && !editor.isSaving && <AdminImageUpload label={`第 ${index + 1} 项轮播图片`} value={slide.imageUrl}
                onChange={(imageUrl) => changeSlide(slide.id, { imageUrl })} />}
            </div>
          </fieldset>
        ))}
        {canWrite && <SettingsSaveConfirmation actionText="保存首页轮播" title="确认保存首页轮播"
          differences={differences} disabled={disabled || editor.feedback?.kind === 'conflict'}
          impactSummary="保存后立即影响移动端首页；停用项、语言不匹配项及未发布的新闻目标不会公开展示。空配置会隐藏轮播。"
          onConfirm={editor.saveChanges} validationIssues={validationIssues} />}
      </div>
    </AdminSettingsPage>
  );
}
