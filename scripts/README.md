# Команда `calc` в CMD

Обратимая настройка **для текущего пользователя**, без администратора.
Системный `C:\Windows\System32\calc.exe` не заменяется.

## Установка

Из корня проекта в CMD:

```bat
app\scripts\setup-calc.cmd Install
calc
```

Установщик копирует готовый EXE из `app\target\x86_64-pc-windows-msvc\release`
в `%LOCALAPPDATA%\Calc1\Command`, создаёт `calc.cmd` и добавляет собственный
вызов в **HKCU\Software\Microsoft\Command Processor\AutoRun**. Вызов добавляет
папку launcher в начало PATH **только запускаемого CMD-процесса**. Порядок и
содержимое системного/пользовательского PATH в реестре не меняются.

Имеющийся AutoRun сохраняется и продолжает выполняться; резервная копия значения
и типа находится в `install-state.json`. Повторная установка не дублирует hook
и может обновить установленный EXE. Перед обновлением закройте калькулятор.

Обычный launcher запускает окно без блокировки CMD и передаёт аргументы:

```bat
calc --lang ru --theme dark
cmd /c calc
```

При стандартной установке `.cmd`-обёртка активирует команду и в текущем CMD.
При запуске PowerShell-скрипта напрямую откройте **новое окно CMD**.

## Проверка и удаление

```bat
app\scripts\setup-calc.cmd Status
where calc
app\scripts\setup-calc.cmd Uninstall
```

`where calc` должен первым показать `%LOCALAPPDATA%\Calc1\Command\calc.cmd`.
После удаления откройте новое окно CMD. Старые процессы могут сохранять прежний
process-local PATH; постоянный PATH не затрагивается.

Удаление восстанавливает прежний AutoRun. Если пользователь добавил другие
команды после установки, удаляется только распознанный hook Calc1. Изменённые
пользователем или занятые файлы не удаляются молча.

## Дополнительные варианты

```powershell
# Только показать план, без изменений:
powershell -NoProfile -ExecutionPolicy Bypass -File app\scripts\Set-CalcCommand.ps1 Install -WhatIf

# Другой источник EXE или каталог установки:
powershell -NoProfile -ExecutionPolicy Bypass -File app\scripts\Set-CalcCommand.ps1 Install -CalculatorPath "D:\Calc\calc1.exe" -InstallDirectory "D:\CalcCommand"
```

При нестандартном каталоге указывайте тот же `-InstallDirectory` при Status и
Uninstall; для активации в уже открытом CMD выполните `call "D:\CalcCommand\cmd-init.cmd"`.

## Границы действия

- Настройка касается команды **`calc`**, не явного `calc.exe`/системного пути,
  Win+R, PowerShell-профиля или ярлыков Windows.
- `cmd /d` отключает AutoRun; mapping в таком процессе возможен только если
  настроенный PATH уже унаследован от родителя либо вызван `cmd-init.cmd` вручную.
- CMD ищет программу в текущем каталоге до PATH. Локальный `calc.exe`/`calc.cmd`
  (например, при работе из System32) имеет приоритет.
- Корпоративные политики, запрещающие AutoRun/PowerShell, не обходятся.
  `-ExecutionPolicy Bypass` действует лишь на процесс запуска скрипта и не
  меняет постоянную политику выполнения.
- Каталог установки с `%`, `!`, `^`, кавычками или переводами строк отвергается
  заранее, чтобы избежать неоднозначного разбора путей CMD.
- Запрашивается только создание скриптов: само наличие этих файлов ещё не
  устанавливает mapping на компьютере.
