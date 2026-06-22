import { useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { PlatformId } from '../types/platform';
import { getPlatformLabel } from '../utils/platformMeta';
import { useGlobalModal } from './useGlobalModal';
import {
  canOpenPlatformFromPackages,
  formatPlatformPackageSize,
  getPlatformPackageFromPackages,
  isHotUpdatePlatformFromPackages,
  isPlatformPackageInstallRequiredFromPackages,
  usePlatformPackageStore,
} from '../stores/usePlatformPackageStore';

type OpenPlatformCallback = () => void;

export function usePlatformPackageInstallPrompt() {
  const { t } = useTranslation();
  const { showModal } = useGlobalModal();
  const platformPackages = usePlatformPackageStore((state) => state.packages);
  const platformPackagesInitialized = usePlatformPackageStore((state) => state.initialized);
  const installPackage = usePlatformPackageStore((state) => state.installPackage);
  const updatePackage = usePlatformPackageStore((state) => state.updatePackage);
  const installPromisesRef = useRef<Map<PlatformId, Promise<void>>>(new Map());

  const ensurePlatformInstalledAndOpen = useCallback((
    platformId: PlatformId,
    onOpen: OpenPlatformCallback,
  ): boolean => {
    if (canOpenPlatformFromPackages(platformPackages, platformPackagesInitialized, platformId)) {
      onOpen();
      return true;
    }

    if (
      !isHotUpdatePlatformFromPackages(platformPackages, platformId)
      || !isPlatformPackageInstallRequiredFromPackages(
        platformPackages,
        platformPackagesInitialized,
        platformId,
      )
    ) {
      return false;
    }

    const platformPackage = getPlatformPackageFromPackages(platformPackages, platformId);
    const platformName = getPlatformLabel(platformId, t);
    const version = platformPackage?.latestVersion || platformPackage?.installedVersion || '--';
    const size = formatPlatformPackageSize(platformPackage?.downloadSizeBytes);
    const isRepair = platformPackage?.installStatus === 'error';
    const isUpdate = platformPackage?.installStatus === 'updateAvailable';

    showModal({
      title: t('platformLayout.packageInstallConfirmTitle', {
        platform: platformName,
        defaultValue: '安装 {{platform}} 平台包',
      }),
      description: t('platformLayout.packageInstallConfirmDesc', {
        platform: platformName,
        version,
        size,
        defaultValue: '{{platform}} 需要先下载平台包后才能打开。版本 {{version}}，大小 {{size}}。',
      }),
      width: 'sm',
      actions: [
        {
          id: 'cancel',
          label: t('common.cancel', '取消'),
          variant: 'secondary',
        },
        {
          id: 'install-and-open',
          label: isRepair
            ? t('platformLayout.packageRepairAndOpen', '修复并打开')
            : isUpdate
              ? t('platformLayout.packageUpdateAndOpen', '更新并打开')
              : t('platformLayout.packageInstallAndOpen', '安装并打开'),
          variant: 'primary',
          onClick: async () => {
            const existingPromise = installPromisesRef.current.get(platformId);
            if (existingPromise) {
              await existingPromise;
              return;
            }

            const installPromise = (async () => {
              const nextState = isUpdate
                ? await updatePackage(platformId)
                : await installPackage(platformId);

              if (typeof window !== 'undefined') {
                window.dispatchEvent(
                  new CustomEvent('agtools:platform-package-changed', {
                    detail: nextState,
                  }),
                );
              }

              if (!nextState.runtimeReady) {
                throw new Error(t('platformLayout.packageInstallNotReady', '平台包已处理，但运行组件尚未就绪'));
              }

              onOpen();
            })().finally(() => {
              installPromisesRef.current.delete(platformId);
            });

            installPromisesRef.current.set(platformId, installPromise);
            await installPromise;
          },
        },
      ],
    });

    return true;
  }, [
    installPackage,
    platformPackages,
    platformPackagesInitialized,
    showModal,
    t,
    updatePackage,
  ]);

  return {
    ensurePlatformInstalledAndOpen,
  };
}
